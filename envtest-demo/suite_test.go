package reconciler

import (
	"context"
	"fmt"
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	scheme "k8s.io/client-go/kubernetes/scheme"
	"log/slog"
	"os"
	"path/filepath"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/envtest"
	"testing"
)

func TestControllers(t *testing.T) {
	RegisterFailHandler(Fail)
	RunSpecs(t, "Controller Suite")
}

var (
	testEnv   *envtest.Environment
	k8sClient client.Client
	ctx       context.Context
	cancel    context.CancelFunc
)

var _ = BeforeSuite(func() {

	ctx, cancel = context.WithCancel(context.TODO())

	SchemeBuilder.Register(&MyDeployment{})
	err := SchemeBuilder.AddToScheme(scheme.Scheme)
	Expect(err).ToNot(HaveOccurred())

	testEnv = &envtest.Environment{
		ErrorIfCRDPathMissing: true,
		CRDDirectoryPaths:     []string{"crd"},
		BinaryAssetsDirectory: getFirstFoundEnvTestBinaryDir(),
	}

	cfg, err := testEnv.Start()
	Expect(err).ToNot(HaveOccurred())
	Expect(cfg).ToNot(BeNil())

	fmt.Printf("api-server: %s, etcd: %s\n", testEnv.ControlPlane.GetAPIServer().Path, testEnv.ControlPlane.Etcd.Path)

	k8sClient, err = client.New(cfg, client.Options{Scheme: scheme.Scheme})
	Expect(err).ToNot(HaveOccurred())
	Expect(k8sClient).ToNot(BeNil())

})

var _ = AfterSuite(func() {
	cancel()
	err := testEnv.Stop()
	Expect(err).ToNot(HaveOccurred())
})

func getFirstFoundEnvTestBinaryDir() string {
	basePath := filepath.Join("bin", "k8s")
	entries, err := os.ReadDir(basePath)
	if err != nil {
		slog.Error("Failed to read directory", "path", basePath, "error", err.Error())
		return ""
	}
	for _, entry := range entries {
		if entry.IsDir() {
			return filepath.Join(basePath, entry.Name())
		}
	}
	return ""
}
