package reconciler

import (
	"context"
	"fmt"
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"log/slog"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

type MyDeploymentReconciler struct {
	client.Client
}

func (r *MyDeploymentReconciler) Reconcile(ctx context.Context, namespacedName types.NamespacedName) (ctrl.Result, error) {
	slog.Info("Reconciling", "name", namespacedName.Name, "namespace", namespacedName.Namespace)
	myDeployment := &MyDeployment{}
	err := r.Get(ctx, namespacedName, myDeployment)
	if err != nil {
		if errors.IsNotFound(err) {
			return ctrl.Result{}, nil
		}
		return ctrl.Result{}, err
	}
	labels := map[string]string{
		"app.kubernetes.io/name": myDeployment.Name,
	}
	deploymentName := fmt.Sprintf("%s-deployment", namespacedName.Name)
	deployment := &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{
			Namespace: namespacedName.Namespace,
			Name:      deploymentName,
		},
		Spec: appsv1.DeploymentSpec{
			Replicas: &myDeployment.Spec.Replicas,
			Selector: &metav1.LabelSelector{
				MatchLabels: labels,
			},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{
					Labels: labels,
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{{
						Name:  "main",
						Image: myDeployment.Spec.Image,
					}},
				},
			},
		},
	}
	err = r.Create(ctx, deployment)
	if err != nil {
		if errors.IsAlreadyExists(err) {
			err = r.Update(ctx, deployment)
		} else {
			return ctrl.Result{}, err
		}
	}
	myDeployment.Status.DeploymentName = deploymentName
	err = r.Status().Update(ctx, myDeployment)
	if err != nil {
		return ctrl.Result{}, err
	}
	return ctrl.Result{}, nil
}
