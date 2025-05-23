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
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"

	appsv1 "k8s.io/api/apps/v1"
)

func ptr[T any](value T) *T {
	return &value
}

var _ = Describe("Deployment Webhook", func() {
	var (
		obj       *appsv1.Deployment
		oldObj    *appsv1.Deployment
		validator DeploymentCustomValidator
		defaulter DeploymentCustomDefaulter
	)

	BeforeEach(func() {
		obj = &appsv1.Deployment{}
		oldObj = &appsv1.Deployment{}
		validator = DeploymentCustomValidator{}
		Expect(validator).NotTo(BeNil(), "Expected validator to be initialized")
		defaulter = DeploymentCustomDefaulter{}
		Expect(defaulter).NotTo(BeNil(), "Expected defaulter to be initialized")
		Expect(oldObj).NotTo(BeNil(), "Expected oldObj to be initialized")
		Expect(obj).NotTo(BeNil(), "Expected obj to be initialized")
	})

	AfterEach(func() {
		oldObj.Annotations = nil
	})

	Context("When creating Deployment under Defaulting Webhook", func() {
		It("Should apply defaults when a required field is empty", func() {
			By("simulating a scenario where defaults should be applied")
			obj.Annotations = nil
			By("calling the Default method to apply defaults")
			_ = defaulter.Default(ctx, obj)
			By("checking that the default values are set")
			Expect(obj.Annotations).To(HaveKeyWithValue("kubernetes.io/description", "Mutated by Default Webhook"))
		})
	})

	Context("When creating or updating Deployment under Validating Webhook", func() {
		It("Should validate updates correctly", func() {
			By("simulating a valid update scenario")
			oldObj.Spec.Replicas = ptr[int32](2)
			obj.Spec.Replicas = ptr[int32](3)
			Expect(validator.ValidateUpdate(ctx, oldObj, obj)).To(BeNil())
		})
		It("Should reject immutable Deployment", func() {
			By("simulating an update with annotation immutable=true")
			oldObj.Annotations = make(map[string]string)
			oldObj.Annotations["immutable"] = "true"
			_, err := validator.ValidateUpdate(ctx, oldObj, obj)
			Expect(err).Should(HaveOccurred())
		})
	})

})
