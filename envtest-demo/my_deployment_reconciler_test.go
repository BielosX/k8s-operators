package reconciler

import (
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	appsv1 "k8s.io/api/apps/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"
)

var _ = Describe("MyDeploymentReconciler", func() {

	namespacedName := types.NamespacedName{
		Namespace: "default",
		Name:      "test-my-deployment",
	}

	var reconciler MyDeploymentReconciler

	BeforeEach(func() {
		reconciler = MyDeploymentReconciler{Client: k8sClient}
	})

	It("Should Reconcile with no error when MyDeployment not found", func() {
		_, err := reconciler.Reconcile(ctx, namespacedName)

		Expect(err).ToNot(HaveOccurred())
	})

	It("Should create a Deployment on Reconcile", func() {
		myDeployment := &MyDeployment{
			ObjectMeta: metav1.ObjectMeta{
				Name:      namespacedName.Name,
				Namespace: namespacedName.Namespace,
			},
			Spec: MyDeploymentSpec{
				Replicas: 2,
				Image:    "nginx:latest",
			},
		}
		Expect(k8sClient.Create(ctx, myDeployment)).To(Succeed())

		_, err := reconciler.Reconcile(ctx, namespacedName)
		Expect(err).ToNot(HaveOccurred())
		Expect(k8sClient.Get(ctx, namespacedName, myDeployment)).To(Succeed())

		deployment := &appsv1.Deployment{}
		Expect(k8sClient.Get(ctx,
			types.NamespacedName{Namespace: namespacedName.Namespace, Name: myDeployment.Status.DeploymentName},
			deployment)).To(Succeed())
		Expect(*deployment.Spec.Replicas).To(Equal(int32(2)))
	})
})
