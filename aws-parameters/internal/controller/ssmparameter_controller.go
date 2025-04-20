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
	"errors"
	"fmt"
	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/ssm"
	ssmtypes "github.com/aws/aws-sdk-go-v2/service/ssm/types"
	k8serrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/controller/controllerutil"
	"time"

	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	logf "sigs.k8s.io/controller-runtime/pkg/log"

	stablev1 "k3builder.com/aws-parameters/api/v1"
)

type SsmParameterAPI interface {
	DeleteParameter(ctx context.Context, params *ssm.DeleteParameterInput, optFns ...func(*ssm.Options)) (*ssm.DeleteParameterOutput, error)
	PutParameter(ctx context.Context, params *ssm.PutParameterInput, optFns ...func(*ssm.Options)) (*ssm.PutParameterOutput, error)
}

// SsmParameterReconciler reconciles a SsmParameter object
type SsmParameterReconciler struct {
	client.Client
	Scheme   *runtime.Scheme
	ParamApi SsmParameterAPI
}

// +kubebuilder:rbac:groups=stable.aws.parameters.com,resources=ssmparameters,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=stable.aws.parameters.com,resources=ssmparameters/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=stable.aws.parameters.com,resources=ssmparameters/finalizers,verbs=update

func getParameterName(ssmParameter *stablev1.SsmParameter) string {
	return fmt.Sprintf("/%s/%s", ssmParameter.Namespace, ssmParameter.Name)
}

func (r *SsmParameterReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	var err error
	logger := logf.FromContext(ctx)

	ssmParameter := &stablev1.SsmParameter{}
	err = r.Get(ctx, req.NamespacedName, ssmParameter)
	if err != nil {
		if k8serrors.IsNotFound(err) {
			logger.Info("SsmParameter not found, probably deleted", req.NamespacedName)
			return ctrl.Result{}, nil
		}
		logger.Error(err, "Unable to load SsmParameter", req.NamespacedName)
		return ctrl.Result{}, err
	}

	finalized, err := r.handleFinalizer(ctx, ssmParameter)
	if err != nil {
		return ctrl.Result{}, err
	}

	if !finalized {
		return r.manageExternalResource(ctx, ssmParameter)
	}
	return ctrl.Result{}, nil
}

func (r *SsmParameterReconciler) manageExternalResource(ctx context.Context,
	ssmParameter *stablev1.SsmParameter) (ctrl.Result, error) {
	var err error
	logger := logf.FromContext(ctx)

	result, err := r.ParamApi.PutParameter(ctx, &ssm.PutParameterInput{
		Name:        aws.String(getParameterName(ssmParameter)),
		Description: &ssmParameter.Spec.Description,
		Overwrite:   aws.Bool(true),
		Value:       &ssmParameter.Spec.Value,
	})
	if err != nil {
		logger.Error(err, "Unable to create/update SsmParameter")
		return ctrl.Result{}, err
	}

	ssmParameter.Status.Version = result.Version
	err = r.Status().Update(ctx, ssmParameter)
	if err != nil {
		logger.Error(err, "Unable to update SsmParameter status", ssmParameter.Name, ssmParameter.Namespace)
		return ctrl.Result{}, err
	}

	return ctrl.Result{RequeueAfter: time.Second * 10}, nil
}

const finalizer = "aws.parameters.com/finalizer"

func (r *SsmParameterReconciler) handleFinalizer(ctx context.Context, ssmParameter *stablev1.SsmParameter) (bool, error) {
	var err error
	logger := logf.FromContext(ctx)
	namespacedName := types.NamespacedName{
		Namespace: ssmParameter.Namespace,
		Name:      ssmParameter.Name,
	}

	if ssmParameter.ObjectMeta.DeletionTimestamp.IsZero() {
		if !controllerutil.ContainsFinalizer(ssmParameter, finalizer) {
			controllerutil.AddFinalizer(ssmParameter, finalizer)
			err = r.Update(ctx, ssmParameter)
			if err != nil {
				logger.Error(err, "Unable to update SsmParameter", namespacedName)
				return false, err
			}
		}
	} else {
		if controllerutil.ContainsFinalizer(ssmParameter, finalizer) {
			err = r.deleteExternalResources(ctx, ssmParameter)
			if err != nil {
				logger.Error(err, "Failed to delete external resources", namespacedName)
				return false, err
			}
			controllerutil.RemoveFinalizer(ssmParameter, finalizer)
			err = r.Update(ctx, ssmParameter)
			if err != nil {
				logger.Error(err, "Unable to update SsmParameter", namespacedName)
				return false, err
			}
			return true, nil
		}
	}
	return false, nil
}

func (r *SsmParameterReconciler) deleteExternalResources(context context.Context,
	ssmParameter *stablev1.SsmParameter) error {
	var err error
	logger := logf.FromContext(context)

	_, err = r.ParamApi.DeleteParameter(context, &ssm.DeleteParameterInput{
		Name: aws.String(getParameterName(ssmParameter)),
	})
	if err != nil {
		var notFound *ssmtypes.ParameterNotFound
		if errors.As(err, &notFound) {
			logger.Info("AWS SSM Parameter not found, nothing to delete",
				"name", ssmParameter.Name, "namespace", ssmParameter.Namespace)
			return nil
		}
		return err
	}
	return nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *SsmParameterReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&stablev1.SsmParameter{}).
		Named("ssmparameter").
		Complete(r)
}
