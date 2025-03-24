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
	"k8s.io/apimachinery/pkg/types"

	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	stablev1 "k3builder.com/exposedapp/api/v1"
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
)

// ExposedAppReconciler reconciles a ExposedApp object
type ExposedAppReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

const mainContainerName = "main"

// +kubebuilder:rbac:groups=stable.k3builder.com,resources=exposedapps,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=stable.k3builder.com,resources=exposedapps/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=stable.k3builder.com,resources=exposedapps/finalizers,verbs=update
// +kubebuilder:rbac:groups=apps,resources=deployments,verbs=create;update;delete;get
// +kubebuilder:rbac:groups=core,resources=services,verbs=create;update;delete;get

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

	deploymentName := exposedApp.Status.DeploymentName

	foundDeployment := &appsv1.Deployment{}
	err = r.Get(ctx, types.NamespacedName{Name: deploymentName, Namespace: namespace}, foundDeployment)
	if err != nil {
		if apierrors.IsNotFound(err) {

		}
	} else {
		*foundDeployment.Spec.Replicas = int32(exposedApp.Spec.Replicas)
		foundDeployment.Spec.Template.Spec.Containers[0].Image = exposedApp.Spec.Image
		foundDeployment.Spec.Template.Spec.Containers[0].Ports[0].ContainerPort = int32(exposedApp.Spec.ContainerPort)
	}

	return ctrl.Result{}, nil
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
