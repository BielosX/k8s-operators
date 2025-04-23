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
	discoveryv1 "k8s.io/api/discovery/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/labels"
	"strings"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	logf "sigs.k8s.io/controller-runtime/pkg/log"
)

// ServiceReconciler reconciles a Service object
type ServiceReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

// +kubebuilder:rbac:groups=core,resources=services,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=core,resources=services/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=core,resources=services/finalizers,verbs=update
// +kubebuilder:rbac:groups=discovery.k8s.io,resources=endpointslices,verbs=get;list;watch

func (r *ServiceReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := logf.FromContext(ctx)

	service := &corev1.Service{}
	err := r.Get(ctx, req.NamespacedName, service)
	if err != nil {
		if errors.IsNotFound(err) {
			logger.Info("Service not found, probably already deleted", "namespacedName", req.NamespacedName)
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}

	if service.Spec.Type == corev1.ServiceTypeLoadBalancer {
		logger.Info("Service type is LoadBalancer, creating external resource", "namespacedName", req.NamespacedName)
		endpointSlices := &discoveryv1.EndpointSliceList{}
		selector, err := labels.Parse(fmt.Sprintf("kubernetes.io/service-name=%s", req.Name))
		if err != nil {
			logger.Error(err, "Unable to parse label selector")
			return ctrl.Result{}, err
		}
		err = r.List(ctx, endpointSlices, &client.ListOptions{
			LabelSelector: selector,
			Namespace:     req.Namespace,
		})
		if err != nil {
			logger.Error(err, "Unable to fetch owned EndpointSlices")
			return ctrl.Result{}, err
		}
		if len(endpointSlices.Items) > 0 {
			logger.Info("Found EndpointSlices", "namespacedName", req.NamespacedName)
			addresses := concatAddresses(endpointSlices.Items)
			logger.Info("Service addresses", "addresses", addresses)
			dns := fmt.Sprintf("%s-%s-424835706.us-west-2.elb.amazonaws.com", req.Namespace, req.Name)
			service.Status.LoadBalancer.Ingress = []corev1.LoadBalancerIngress{{
				Hostname: dns,
			}}
			err = r.Status().Update(ctx, service)
			if err != nil {
				logger.Error(err, "Unable to update Service status", "namespacedName", req.NamespacedName)
				return ctrl.Result{}, err
			}
		} else {
			logger.Info("EndpointSlices not found", "namespacedName", req.NamespacedName)
		}
	} else {
		logger.Info("Service type is not LoadBalancer, nothing to do", "namespacedName", req.NamespacedName)
		return ctrl.Result{}, nil
	}

	return ctrl.Result{}, nil
}

func concatAddresses(endpointSlices []discoveryv1.EndpointSlice) string {
	var addresses []string
	for _, slice := range endpointSlices {
		for _, endpoint := range slice.Endpoints {
			for _, address := range endpoint.Addresses {
				addresses = append(addresses, address)
			}
		}
	}
	return strings.Join(addresses, ",")
}

// SetupWithManager sets up the controller with the Manager.
func (r *ServiceReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Service{}).
		Named("service").
		Owns(&discoveryv1.EndpointSlice{}).
		Complete(r)
}
