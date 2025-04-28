package reconciler

import (
	"context"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

type MyDeploymentReconciler struct {
	client.Client
}

func (r *MyDeploymentReconciler) Reconcile(ctx context.Context, namespacedName types.NamespacedName) (ctrl.Result, error) {

	return ctrl.Result{}, nil
}
