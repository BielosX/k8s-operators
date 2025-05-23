package main

import (
	"context"
	"fmt"
	appsv1 "k8s.io/api/apps/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/apimachinery/pkg/watch"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
	"log/slog"
	"os"
	"reflect"
	"sync"
)

type K8sClient struct {
	clientSet *kubernetes.Clientset
	mutex     sync.Mutex
	cache     map[types.NamespacedName]metav1.ObjectMeta
}

func NewK8sClient() (*K8sClient, error) {
	config, err := rest.InClusterConfig()
	if err != nil {
		return nil, err
	}
	clientSet, err := kubernetes.NewForConfig(config)
	if err != nil {
		return nil, err
	}
	return &K8sClient{clientSet: clientSet, cache: make(map[types.NamespacedName]metav1.ObjectMeta)}, nil
}

type DeploymentUpdater interface {
	UpdateDeployment(ctx context.Context, deployment *appsv1.Deployment) (*appsv1.Deployment, error)
}

func (c *K8sClient) UpdateDeployment(
	ctx context.Context,
	deployment *appsv1.Deployment,
) (*appsv1.Deployment, error) {
	defer c.mutex.Unlock()
	c.mutex.Lock()
	result, err := c.clientSet.AppsV1().
		Deployments(deployment.Namespace).
		Update(ctx, deployment, metav1.UpdateOptions{})
	if err != nil {
		return nil, err
	}
	c.cache[types.NamespacedName{
		Namespace: deployment.Namespace,
		Name:      deployment.Name,
	}] = result.ObjectMeta
	return result, nil
}

func (c *K8sClient) handleEvent(ctx context.Context, event *watch.Event, reconciler *Reconciler) {
	defer c.mutex.Unlock()
	c.mutex.Lock()
	slog.Info(fmt.Sprintf("Received an event of type %s", event.Type))
	if event.Type == watch.Added || event.Type == watch.Modified {
		deployment := event.Object.(*appsv1.Deployment)
		entry, ok := c.cache[types.NamespacedName{
			Namespace: deployment.Namespace,
			Name:      deployment.Name,
		}]
		if !ok {
			slog.Info(
				fmt.Sprintf(
					"No cache entry for %s %s, reconciling",
					deployment.Name,
					deployment.Namespace,
				),
			)
			go reconciler.Reconcile(ctx, deployment)
		} else {
			if deployment.ResourceVersion != entry.ResourceVersion { // Labels, Spec or Status changed
				slog.Info(fmt.Sprintf("Received version %s, cached version %s for %s %s",
					deployment.ResourceVersion,
					entry.ResourceVersion,
					deployment.Name,
					deployment.Namespace))
				if reflect.DeepEqual(deployment.ObjectMeta.Labels, entry.Labels) { // Labels didn't change, only Status or Spec
					if deployment.Generation != entry.Generation { // Spec changed
						slog.Info(fmt.Sprintf("Spec updated for %s %s, Reconcile", deployment.Name, deployment.Namespace))
						go reconciler.Reconcile(ctx, deployment)
					} else { // Only Status changed
						slog.Info(fmt.Sprintf("Status updated for %s %s, Skip", deployment.Name, deployment.Namespace))
					}
				} else { // Cached Labels not equal to one received
					slog.Info(fmt.Sprintf("Labels updated for %s %s, Reconcile",
						deployment.Name,
						deployment.Namespace))
					go reconciler.Reconcile(ctx, deployment)
				}
			}
		}
	}
}

func (c *K8sClient) WatchDeployments(ctx context.Context, reconciler *Reconciler) error {
	result, err := c.clientSet.AppsV1().Deployments("").Watch(ctx, metav1.ListOptions{})
	if err != nil {
		return err
	}
	defer result.Stop()
	for event := range result.ResultChan() {
		c.handleEvent(ctx, &event, reconciler)
	}
	return nil
}

type Reconciler struct {
	updater DeploymentUpdater
}

func NewReconciler(updater DeploymentUpdater) *Reconciler {
	return &Reconciler{updater: updater}
}

func (r *Reconciler) Reconcile(ctx context.Context, deployment *appsv1.Deployment) {
	slog.Info(fmt.Sprintf("Reconciling %s %s", deployment.Name, deployment.Namespace))
	if deployment.Labels == nil {
		deployment.Labels = make(map[string]string)
	}
	replicas := *deployment.Spec.Replicas
	if replicas > 1 {
		deployment.Labels["multipleReplicas"] = "true"
	} else {
		deployment.Labels["multipleReplicas"] = "false"
	}
	_, err := r.updater.UpdateDeployment(ctx, deployment)
	if err != nil {
		slog.Error(
			fmt.Sprintf(
				"Failed to update Deployment %s %s. Error: %s",
				deployment.Name,
				deployment.Namespace,
				err.Error(),
			),
		)
	}
}

func main() {
	slog.Info("Starting controller")
	client, err := NewK8sClient()
	if err != nil {
		slog.Error(fmt.Sprintf("Unable to create K8sClient, Reason: %s", err.Error()))
		os.Exit(1)
	}
	/*
		K8sClient UpdateDeployment is a pointer receiver (it's declared for *K8sClient)
		So it needs to be passed as a pointer. Everything is safe (no mutex copy)
	*/
	reconciler := NewReconciler(client)
	err = client.WatchDeployments(context.TODO(), reconciler)
	if err != nil {
		slog.Error(fmt.Sprintf("Failed to watch Deployments, Reason: %s", err.Error()))
		os.Exit(1)
	}
}
