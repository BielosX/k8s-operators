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

package v1

import (
	"context"
	"fmt"

	"github.com/prometheus/client_golang/prometheus"
	appsv1 "k8s.io/api/apps/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	logf "sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/metrics"
	"sigs.k8s.io/controller-runtime/pkg/webhook"
	"sigs.k8s.io/controller-runtime/pkg/webhook/admission"
)

// nolint:unused
// log is for logging in this package.
var deploymentlog = logf.Log.WithName("deployment-resource")

var (
	defaultCalls = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "k3builder_core_default_calls",
		Help: "Total number of Default func calls",
	})
	validateUpdateCalls = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "k3builder_core_validate_update_calls",
		Help: "Total number of ValidateUpdate calls",
	})
)

// SetupDeploymentWebhookWithManager registers the webhook for Deployment in the manager.
func SetupDeploymentWebhookWithManager(mgr ctrl.Manager) error {
	metrics.Registry.MustRegister(defaultCalls, validateUpdateCalls)
	return ctrl.NewWebhookManagedBy(mgr).For(&appsv1.Deployment{}).
		WithValidator(&DeploymentCustomValidator{}).
		WithDefaulter(&DeploymentCustomDefaulter{}).
		Complete()
}

// +kubebuilder:webhook:path=/mutate-apps-v1-deployment,mutating=true,failurePolicy=fail,sideEffects=None,groups=apps,resources=deployments,verbs=create,versions=v1,name=mdeployment-v1.kb.io,admissionReviewVersions=v1

// DeploymentCustomDefaulter struct is responsible for setting default values on the custom resource of the
// Kind Deployment when those are created or updated.
//
// NOTE: The +kubebuilder:object:generate=false marker prevents controller-gen from generating DeepCopy methods,
// as it is used only for temporary operations and does not need to be deeply copied.
type DeploymentCustomDefaulter struct{}

var _ webhook.CustomDefaulter = &DeploymentCustomDefaulter{}

// Default implements webhook.CustomDefaulter so a webhook will be registered for the Kind Deployment.
func (d *DeploymentCustomDefaulter) Default(_ context.Context, obj runtime.Object) error {
	deployment, ok := obj.(*appsv1.Deployment)
	defaultCalls.Inc()

	if !ok {
		return fmt.Errorf("expected an Deployment object but got %T", obj)
	}
	deploymentlog.Info("Defaulting for Deployment", "name", deployment.GetName())

	if deployment.Annotations == nil {
		deployment.Annotations = make(map[string]string)
	}
	deployment.Annotations["kubernetes.io/description"] = "Mutated by Default Webhook"

	return nil
}

// NOTE: The 'path' attribute must follow a specific pattern and should not be modified directly here.
// Modifying the path for an invalid path can cause API server errors; failing to locate the webhook.
// +kubebuilder:webhook:path=/validate-apps-v1-deployment,mutating=false,failurePolicy=fail,sideEffects=None,groups=apps,resources=deployments,verbs=update,versions=v1,name=vdeployment-v1.kb.io,admissionReviewVersions=v1

// DeploymentCustomValidator struct is responsible for validating the Deployment resource
// when it is created, updated, or deleted.
//
// NOTE: The +kubebuilder:object:generate=false marker prevents controller-gen from generating DeepCopy methods,
// as this struct is used only for temporary operations and does not need to be deeply copied.
type DeploymentCustomValidator struct{}

var _ webhook.CustomValidator = &DeploymentCustomValidator{}

// ValidateCreate implements webhook.CustomValidator so a webhook will be registered for the type Deployment.
func (v *DeploymentCustomValidator) ValidateCreate(_ context.Context, _ runtime.Object) (admission.Warnings, error) {
	return nil, nil
}

// ValidateUpdate implements webhook.CustomValidator so a webhook will be registered for the type Deployment.
func (v *DeploymentCustomValidator) ValidateUpdate(_ context.Context, oldObj, _ runtime.Object) (admission.Warnings, error) {
	deployment, ok := oldObj.(*appsv1.Deployment)
	validateUpdateCalls.Inc()
	if !ok {
		return nil, fmt.Errorf("expected a Deployment object for the oldObj but got %T", oldObj)
	}
	deploymentlog.Info("Validation for Deployment upon update", "name", deployment.GetName())

	if deployment.Annotations != nil {
		value, ok := deployment.Annotations["immutable"]
		if ok && value == "true" {
			return nil, fmt.Errorf("deployment marked as immutable, unable to update. Delete first, then recreate")
		}
	}

	return nil, nil
}

// ValidateDelete implements webhook.CustomValidator so a webhook will be registered for the type Deployment.
func (v *DeploymentCustomValidator) ValidateDelete(_ context.Context, _ runtime.Object) (admission.Warnings, error) {
	return nil, nil
}
