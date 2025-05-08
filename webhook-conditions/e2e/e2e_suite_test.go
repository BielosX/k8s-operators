package e2e_test

import (
	"log"
	"net/http"
	"os/exec"
	"testing"
	"time"

	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
)

func TestE2e(t *testing.T) {
	RegisterFailHandler(Fail)
	RunSpecs(t, "E2e Suite")
}

var cmd = exec.Command("../target/main")

var _ = BeforeSuite(func() {
	err := cmd.Start()
	if err != nil {
		log.Fatal(err)
	}
	By("Waiting for server to start")
	Eventually(func(g Gomega) {
		resp, err := http.Get("http://localhost:8080/healthz")
		g.Expect(err).ToNot(HaveOccurred())
		g.Expect(resp.StatusCode).To(Equal(http.StatusOK))
	}).WithTimeout(time.Second * 20).WithPolling(time.Second * 2).Should(Succeed())
})

var _ = AfterSuite(func() {
	err := cmd.Process.Kill()
	if err != nil {
		log.Fatal(err)
	}
})
