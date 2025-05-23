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
	"github.com/aws/aws-sdk-go-v2/service/ssm"
	ssmtypes "github.com/aws/aws-sdk-go-v2/service/ssm/types"
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"

	stablev1 "k3builder.com/aws-parameters/api/v1"
)

type MockSsmParameterAPI struct {
	DeleteParameterMock func(params *ssm.DeleteParameterInput) (*ssm.DeleteParameterOutput, error)
	PutParameterMock    func(params *ssm.PutParameterInput) (*ssm.PutParameterOutput, error)
}

func (m *MockSsmParameterAPI) DeleteParameter(_ context.Context, params *ssm.DeleteParameterInput, _ ...func(*ssm.Options)) (*ssm.DeleteParameterOutput, error) {
	return m.DeleteParameterMock(params)
}

func (m *MockSsmParameterAPI) PutParameter(_ context.Context, params *ssm.PutParameterInput, _ ...func(*ssm.Options)) (*ssm.PutParameterOutput, error) {
	return m.PutParameterMock(params)
}

var _ = Describe("SsmParameter Controller", func() {
	Context("When reconciling a resource", func() {
		const resourceName = "test-resource"

		ctx := context.Background()

		typeNamespacedName := types.NamespacedName{
			Name:      resourceName,
			Namespace: "default",
		}

		ssmMock := &MockSsmParameterAPI{}

		var reconciler SsmParameterReconciler
		var ssmParameter *stablev1.SsmParameter

		BeforeEach(func() {
			ssmParameter = &stablev1.SsmParameter{
				ObjectMeta: metav1.ObjectMeta{
					Name:      typeNamespacedName.Name,
					Namespace: typeNamespacedName.Namespace,
				},
				Spec: stablev1.SsmParameterSpec{
					Description: "Demo Parameter",
					Value:       "Some Value",
				},
			}
			By("creating the custom resource for the Kind SsmParameter")
			Expect(k8sClient.Create(ctx, ssmParameter)).To(Succeed())

			ssmMock.DeleteParameterMock = func(_ *ssm.DeleteParameterInput) (*ssm.DeleteParameterOutput, error) {
				return &ssm.DeleteParameterOutput{}, nil
			}
			ssmMock.PutParameterMock = func(_ *ssm.PutParameterInput) (*ssm.PutParameterOutput, error) {
				return &ssm.PutParameterOutput{Tier: ssmtypes.ParameterTierStandard, Version: 1}, nil
			}

			reconciler = SsmParameterReconciler{
				Client:   k8sClient,
				Scheme:   k8sClient.Scheme(),
				ParamApi: ssmMock,
			}
		})

		AfterEach(func() {
			err := k8sClient.Get(ctx, typeNamespacedName, ssmParameter)
			if err == nil {
				By("Remove SsmParameter finalizers")
				ssmParameter.Finalizers = []string{}
				Expect(k8sClient.Update(ctx, ssmParameter)).To(Succeed())
				By("Cleanup the specific resource instance SsmParameter")
				Expect(k8sClient.Delete(ctx, ssmParameter)).To(Succeed())
			}

		})

		It("should successfully reconcile the resource", func() {
			By("Reconciling the created resource")
			_, err := reconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
			err = k8sClient.Get(ctx, typeNamespacedName, ssmParameter)
			Expect(err).NotTo(HaveOccurred())
			Expect(ssmParameter.Status.Version).To(Equal(int64(1)))
			Expect(ssmParameter.Finalizers).To(ContainElement("aws.parameters.com/finalizer"))
		})

		It("should successfully reconcile when external resource already deleted", func() {
			ssmMock.DeleteParameterMock = func(_ *ssm.DeleteParameterInput) (*ssm.DeleteParameterOutput, error) {
				return nil, &ssmtypes.ParameterNotFound{}
			}
			By("Setting finalizer")
			ssmParameter.Finalizers = []string{"aws.parameters.com/finalizer"}
			Expect(k8sClient.Update(ctx, ssmParameter)).To(Succeed())
			By("Deleting object with finalizers")
			Expect(k8sClient.Delete(ctx, ssmParameter)).NotTo(HaveOccurred())

			Expect(k8sClient.Get(ctx, typeNamespacedName, ssmParameter)).To(Succeed())
			Expect(ssmParameter.DeletionTimestamp).ToNot(BeNil())

			By("Reconciling the created resource")
			_, err := reconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
		})

		It("should remove finalizer on deletion", func() {
			By("Reconciling the created resource")
			_, err := reconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())

			Expect(k8sClient.Get(ctx, typeNamespacedName, ssmParameter)).To(Succeed())
			Expect(ssmParameter.Finalizers).To(ContainElement("aws.parameters.com/finalizer"))

			Expect(k8sClient.Delete(ctx, ssmParameter)).To(Succeed())
			Expect(k8sClient.Get(ctx, typeNamespacedName, ssmParameter)).To(Succeed())
			Expect(ssmParameter.DeletionTimestamp).ToNot(BeNil())

			By("Reconciling the deleted resource")
			_, err = reconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			err = k8sClient.Get(ctx, typeNamespacedName, ssmParameter)
			Expect(errors.IsNotFound(err)).To(BeTrue())
		})

		It("should use name prefix when provided", func() {
			reconciler.Prefix = "/prefix"
			ssmMock.PutParameterMock = func(input *ssm.PutParameterInput) (*ssm.PutParameterOutput, error) {
				Expect(*input.Name).To(Equal("/prefix/default/test-resource"))
				return &ssm.PutParameterOutput{Tier: ssmtypes.ParameterTierStandard, Version: 1}, nil
			}

			By("Reconciling the created resource")
			_, err := reconciler.Reconcile(ctx, reconcile.Request{
				NamespacedName: typeNamespacedName,
			})
			Expect(err).NotTo(HaveOccurred())
		})
	})
})
