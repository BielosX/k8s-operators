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
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/tools/record"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	stablev1 "k3builder.com/exposedapp/api/v1"
	"k8s.io/utils/ptr"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

var _ = Describe("ExposedApp Controller", func() {
	Context("When reconciling a resource", func() {
		const resourceName = "test-resource"

		ctx := context.Background()

		typeNamespacedName := types.NamespacedName{
			Name:      resourceName,
			Namespace: "default",
		}
		exposedapp := &stablev1.ExposedApp{}

		var controllerReconciler ExposedAppReconciler

		BeforeEach(func() {
			By("creating the custom resource for the Kind ExposedApp")
			err := k8sClient.Get(ctx, typeNamespacedName, exposedapp)
			if err != nil && errors.IsNotFound(err) {
				var nodePort stablev1.Port = 30000
				resource := &stablev1.ExposedApp{
					ObjectMeta: metav1.ObjectMeta{
						Name:      resourceName,
						Namespace: "default",
					},
					Spec: stablev1.ExposedAppSpec{
						Replicas:      2,
						Image:         "nginx:latest",
						Protocol:      "TCP",
						Port:          1234,
						ContainerPort: 80,
						NodePort:      &nodePort,
						ServiceType:   "NodePort",
					},
				}
				Expect(k8sClient.Create(ctx, resource)).To(Succeed())
			}
			By("creating ExposedAppReconciler")
			controllerReconciler = ExposedAppReconciler{
				Client:   k8sClient,
				Scheme:   k8sClient.Scheme(),
				Recorder: record.NewFakeRecorder(10),
			}
		})

		AfterEach(func() {
			resource := &stablev1.ExposedApp{}
			err := k8sClient.Get(ctx, typeNamespacedName, resource)
			if err == nil {
				By("Cleanup the specific resource instance ExposedApp")
				Expect(k8sClient.Delete(ctx, resource, &client.DeleteOptions{
					PropagationPolicy: ptr.To(metav1.DeletePropagationForeground),
				})).To(Succeed())
			}

		})
		It("should successfully reconcile the resource", func() {
			By("Reconciling the created resource")
			_, err := controllerReconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
			resource := &stablev1.ExposedApp{}
			err = k8sClient.Get(ctx, typeNamespacedName, resource)
			Expect(err).NotTo(HaveOccurred())
			Expect(resource.Status.DeploymentName).To(Equal("test-resource-default-deployment"))
			Expect(resource.Status.ServiceName).To(Equal("test-resource-default-service"))
		})
		It("should successfully create deployment with proper parameters", func() {
			By("Reconciling the created resource")
			_, err := controllerReconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
			resource := &stablev1.ExposedApp{}
			err = k8sClient.Get(ctx, typeNamespacedName, resource)
			Expect(err).NotTo(HaveOccurred())

			deployment := &appsv1.Deployment{}
			err = k8sClient.Get(ctx, types.NamespacedName{
				Namespace: typeNamespacedName.Namespace,
				Name:      resource.Status.DeploymentName,
			}, deployment)
			Expect(err).NotTo(HaveOccurred())
			Expect(*deployment.Spec.Replicas).To(Equal(resource.Spec.Replicas))
			Expect(deployment.Spec.Template.Spec.Containers[0].Image).To(Equal(resource.Spec.Image))
			Expect(deployment.Spec.Template.Spec.Containers[0].Ports[0].ContainerPort).To(Equal(int32(resource.Spec.ContainerPort)))
			Expect(deployment.Spec.Template.Spec.Containers[0].Ports[0].Protocol).To(Equal(corev1.Protocol(resource.Spec.Protocol)))
			Expect(deployment.OwnerReferences[0].Name).To(Equal(resource.Name))
		})
		It("should successfully create service with proper parameters", func() {
			By("Reconciling the created resource")
			_, err := controllerReconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
			resource := &stablev1.ExposedApp{}
			err = k8sClient.Get(ctx, typeNamespacedName, resource)
			Expect(err).NotTo(HaveOccurred())

			service := &corev1.Service{}
			err = k8sClient.Get(ctx, types.NamespacedName{
				Namespace: typeNamespacedName.Namespace,
				Name:      resource.Status.ServiceName,
			}, service)
			Expect(err).NotTo(HaveOccurred())
			Expect(service.Spec.Type).To(Equal(corev1.ServiceType(resource.Spec.ServiceType)))
			Expect(service.Spec.Ports[0].Port).To(Equal(int32(resource.Spec.Port)))
			Expect(service.Spec.Ports[0].NodePort).To(Equal(int32(*resource.Spec.NodePort)))
			Expect(service.Spec.Ports[0].Protocol).To(Equal(corev1.Protocol(resource.Spec.Protocol)))
			Expect(service.OwnerReferences[0].Name).To(Equal(resource.Name))
		})
		It("should successfully update deployment with proper parameters", func() {
			By("Reconciling the created resource")
			_, err := controllerReconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
			resource := &stablev1.ExposedApp{}
			err = k8sClient.Get(ctx, typeNamespacedName, resource)
			Expect(err).NotTo(HaveOccurred())
			Expect(resource.Spec.Replicas).To(Equal(int32(2)))

			By("Fetching the deployment and updating the replicas")
			deployment := &appsv1.Deployment{}
			err = k8sClient.Get(ctx, types.NamespacedName{
				Namespace: typeNamespacedName.Namespace,
				Name:      resource.Status.DeploymentName,
			}, deployment)
			Expect(err).NotTo(HaveOccurred())
			Expect(*deployment.Spec.Replicas).To(Equal(resource.Spec.Replicas))

			resource.Spec.Replicas = 3
			Expect(k8sClient.Update(ctx, resource)).To(Succeed())
			By("Reconciling the updated resource")
			_, err = controllerReconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})

			By("Fetching the deployment and updating the replicas")
			deployment = &appsv1.Deployment{}
			err = k8sClient.Get(ctx, types.NamespacedName{
				Namespace: typeNamespacedName.Namespace,
				Name:      resource.Status.DeploymentName,
			}, deployment)
			Expect(err).NotTo(HaveOccurred())
			Expect(*deployment.Spec.Replicas).To(Equal(int32(3)))
		})
		It("Should set ownerReference on the created resources", func() {
			By("Reconciling the created resource")
			_, err := controllerReconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
			resource := &stablev1.ExposedApp{}
			err = k8sClient.Get(ctx, typeNamespacedName, resource)
			Expect(err).NotTo(HaveOccurred())

			deployment := &appsv1.Deployment{}
			err = k8sClient.Get(ctx, types.NamespacedName{
				Namespace: typeNamespacedName.Namespace,
				Name:      resource.Status.DeploymentName,
			}, deployment)
			Expect(err).NotTo(HaveOccurred())
			By("Checking the ownerReference on the deployment")
			Expect(deployment.OwnerReferences[0].Name).To(Equal(resource.Name))
			Expect(deployment.OwnerReferences[0].Kind).To(Equal("ExposedApp"))
			Expect(deployment.OwnerReferences[0].UID).To(Equal(resource.UID))
			Expect(*deployment.OwnerReferences[0].Controller).To(Equal(true))
			Expect(*deployment.OwnerReferences[0].BlockOwnerDeletion).To(Equal(true))

			service := &corev1.Service{}
			err = k8sClient.Get(ctx, types.NamespacedName{
				Namespace: typeNamespacedName.Namespace,
				Name:      resource.Status.ServiceName,
			}, service)
			Expect(err).NotTo(HaveOccurred())
			By("Checking the ownerReference on the service")
			Expect(service.OwnerReferences[0].Name).To(Equal(resource.Name))
			Expect(service.OwnerReferences[0].Kind).To(Equal("ExposedApp"))
			Expect(service.OwnerReferences[0].UID).To(Equal(resource.UID))
			Expect(*service.OwnerReferences[0].Controller).To(Equal(true))
			Expect(*service.OwnerReferences[0].BlockOwnerDeletion).To(Equal(true))
		})
	})
})
