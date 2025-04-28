package reconciler

import (
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	"k8s.io/apimachinery/pkg/types"
)

var _ = Describe("MyDeploymentReconciler", func() {

	namespacedName := types.NamespacedName{
		Namespace: "example",
		Name:      "test-my-deployment",
	}

	var reconciler MyDeploymentReconciler

	BeforeEach(func() {
		reconciler = MyDeploymentReconciler{k8sClient}
	})

	It("Should Reconcile with no error when MyDeployment not found", func() {
		_, err := reconciler.Reconcile(ctx, namespacedName)

		Expect(err).ToNot(HaveOccurred())
	})
})
