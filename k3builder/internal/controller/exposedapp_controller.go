/*
Copyright 2025.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package controller

import (
	"context"
	"fmt"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"

	"k8s.io/apimachinery/pkg/api/meta"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	stablev1 "k3builder.com/exposedapp/api/v1"
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// ExposedAppReconciler reconciles a ExposedApp object
type ExposedAppReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	Recorder record.EventRecorder
	PodName  string
}

const mainContainerName = "main"

// +kubebuilder:rbac:groups=stable.k3builder.com,resources=exposedapps,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=stable.k3builder.com,resources=exposedapps/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=stable.k3builder.com,resources=exposedapps/finalizers,verbs=update
// +kubebuilder:rbac:groups=apps,resources=deployments,verbs=create;update;delete;get;watch;list
// +kubebuilder:rbac:groups=core,resources=services,verbs=create;update;delete;get;watch;list
// +kubebuilder:rbac:groups=core,resources=events,verbs=create;patch

func (r *ExposedAppReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	exposedApp := &stablev1.ExposedApp{}
	namespace := req.NamespacedName.Namespace
	err := r.Get(ctx, req.NamespacedName, exposedApp)
	if err != nil {
		if apierrors.IsNotFound(err) {
			logger.Info("ExposedApp resource not found, probably deleted.")
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	if len(exposedApp.Status.Conditions) == 0 {
		err = r.InitStatus(ctx, exposedApp)
		if err != nil {
			return ctrl.Result{}, err
		}
	}

	deploymentName := fmt.Sprintf("%s-%s-deployment", exposedApp.Name, exposedApp.Namespace)
	serviceName := fmt.Sprintf("%s-%s-service", exposedApp.Name, exposedApp.Namespace)

	deployment := &appsv1.Deployment{}
	err = r.Get(ctx, types.NamespacedName{Name: deploymentName, Namespace: namespace}, deployment)
	if err != nil {
		if apierrors.IsNotFound(err) {
			deployment, err = r.Deployment(exposedApp, deploymentName)
			if err != nil {
				return ctrl.Result{}, err
			}
			err = r.Create(ctx, deployment)
			if err != nil {
				return ctrl.Result{}, err
			}
			r.Recorder.Eventf(exposedApp, "Normal",
				"DeploymentCreated",
				"Deployment %s created",
				deploymentName)
		} else {
			return ctrl.Result{}, err
		}
	} else {
		deployment.Spec.Replicas = &exposedApp.Spec.Replicas
		deployment.Spec.Template.Spec.Containers[0].Image = exposedApp.Spec.Image
		deployment.Spec.Template.Spec.Containers[0].Ports[0].ContainerPort = exposedApp.Spec.ContainerPort
		deployment.Spec.Template.Spec.Containers[0].Ports[0].Protocol = corev1.Protocol(exposedApp.Spec.Protocol)
		err = r.Update(ctx, deployment)
		if err != nil {
			return ctrl.Result{}, err
		}
		r.Recorder.Eventf(exposedApp, "Normal",
			"DeploymentUpdated",
			"Deployment %s updated",
			deploymentName)
	}
	meta.SetStatusCondition(&exposedApp.Status.Conditions, metav1.Condition{
		Status:  metav1.ConditionTrue,
		Reason:  "Provisioned",
		Message: "Deployment ready",
		Type:    "DeploymentReady",
	})
	err = r.Status().Update(ctx, exposedApp)
	if err != nil {
		logger.Error(err, "Unable to update ExposedApp Status")
		return ctrl.Result{}, err
	}

	service := &corev1.Service{}
	err = r.Get(ctx, types.NamespacedName{Name: serviceName, Namespace: namespace}, service)
	if err != nil {
		if apierrors.IsNotFound(err) {
			service, err = r.Service(exposedApp, deployment.Spec.Template.ObjectMeta.Labels, serviceName)
			if err != nil {
				return ctrl.Result{}, err
			}
			err = r.Create(ctx, service)
			if err != nil {
				return ctrl.Result{}, err
			}
			r.Recorder.Eventf(exposedApp, "Normal",
				"ServiceCreated",
				"Service %s created",
				serviceName)
		} else {
			return ctrl.Result{}, err
		}
	} else {
		service.Spec.Ports[0].Port = exposedApp.Spec.Port
		service.Spec.Ports[0].Protocol = corev1.Protocol(exposedApp.Spec.Protocol)
		if exposedApp.Spec.NodePort != nil {
			service.Spec.Ports[0].NodePort = *exposedApp.Spec.NodePort
		}
		if len(exposedApp.Spec.ServiceType) != 0 {
			service.Spec.Type = corev1.ServiceType(exposedApp.Spec.ServiceType)
		}
		err = r.Update(ctx, service)
		if err != nil {
			return ctrl.Result{}, err
		}
		r.Recorder.Eventf(exposedApp, "Normal",
			"ServiceUpdated",
			"Service %s updated",
			serviceName)
	}

	exposedApp.Status.ServiceName = serviceName
	exposedApp.Status.DeploymentName = deploymentName
	exposedApp.Status.LastUpdateBy = r.PodName
	exposedApp.Status.Ready = fmt.Sprintf("%d/%d", deployment.Status.ReadyReplicas, deployment.Status.ReadyReplicas)
	meta.SetStatusCondition(&exposedApp.Status.Conditions, metav1.Condition{
		Status:  metav1.ConditionTrue,
		Reason:  "Provisioned",
		Message: "Service ready",
		Type:    "ServiceReady",
	})
	err = r.Status().Update(ctx, exposedApp)
	if err != nil {
		logger.Error(err, "Unable to update ExposedApp Status")
		return ctrl.Result{}, err
	}

	// RequeueAfter not required here as Deployment is a dependent resource and this controller watches for changes
	// RequeueAfter might be useful for managing external resources, like AWS ALB to poll the status
	return ctrl.Result{}, nil
}

func (r *ExposedAppReconciler) InitStatus(ctx context.Context, exposedApp *stablev1.ExposedApp) error {
	logger := log.FromContext(ctx)
	meta.SetStatusCondition(&exposedApp.Status.Conditions, metav1.Condition{
		Status:  metav1.ConditionFalse,
		Reason:  "Provisioning",
		Message: "Deployment not ready",
		Type:    "DeploymentReady",
	})
	meta.SetStatusCondition(&exposedApp.Status.Conditions, metav1.Condition{
		Status:  metav1.ConditionFalse,
		Reason:  "Provisioning",
		Message: "Service not ready",
		Type:    "ServiceReady",
	})
	err := r.Status().Update(ctx, exposedApp)
	if err != nil {
		logger.Error(err, "Unable to update ExposedApp Status")
		return err
	}
	return nil
}

func (r *ExposedAppReconciler) Service(exposedApp *stablev1.ExposedApp, podLabels map[string]string, serviceName string) (*corev1.Service, error) {
	servicePort := corev1.ServicePort{
		Protocol: corev1.Protocol(exposedApp.Spec.Protocol),
		Port:     exposedApp.Spec.Port,
	}
	if exposedApp.Spec.NodePort != nil {
		servicePort.NodePort = *exposedApp.Spec.NodePort
	}
	service := &corev1.Service{
		ObjectMeta: metav1.ObjectMeta{
			Namespace: exposedApp.Namespace,
			Name:      serviceName,
		},
		Spec: corev1.ServiceSpec{
			Selector: podLabels,
			Ports:    []corev1.ServicePort{servicePort},
		},
	}
	if len(exposedApp.Spec.ServiceType) != 0 {
		service.Spec.Type = corev1.ServiceType(exposedApp.Spec.ServiceType)
	}
	// Set ExposedApp as owner
	err := ctrl.SetControllerReference(exposedApp, service, r.Scheme)
	if err != nil {
		return nil, err
	}
	return service, nil
}

func (r *ExposedAppReconciler) Deployment(exposedApp *stablev1.ExposedApp, deploymentName string) (*appsv1.Deployment, error) {
	labels := map[string]string{"app.kubernetes.io/name": fmt.Sprintf("%s-%s", exposedApp.Name, exposedApp.Namespace)}
	deployment := &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{
			Namespace: exposedApp.Namespace,
			Name:      deploymentName,
		},
		Spec: appsv1.DeploymentSpec{
			Replicas: &exposedApp.Spec.Replicas,
			Selector: &metav1.LabelSelector{
				MatchLabels: labels,
			},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{
					Labels: labels,
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{{
						Image: exposedApp.Spec.Image,
						Name:  mainContainerName,
						Ports: []corev1.ContainerPort{{
							ContainerPort: exposedApp.Spec.ContainerPort,
							Protocol:      corev1.Protocol(exposedApp.Spec.Protocol),
						}},
					}},
				},
			},
		},
	}
	// Set ExposedApp as owner
	err := ctrl.SetControllerReference(exposedApp, deployment, r.Scheme)
	if err != nil {
		return nil, err
	}
	return deployment, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *ExposedAppReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&stablev1.ExposedApp{}).
		Named("exposedapp").
		Owns(&appsv1.Deployment{}).
		Owns(&corev1.Service{}).
		Complete(r)
}
