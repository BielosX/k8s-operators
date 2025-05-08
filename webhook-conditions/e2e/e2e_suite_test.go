package e2e_test

import (
	"bufio"
	"log"
	"os/exec"
	"testing"

	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
)

func TestE2e(t *testing.T) {
	RegisterFailHandler(Fail)
	RunSpecs(t, "E2e Suite")
}

var cmd = exec.Command("../target/main")

var _ = BeforeSuite(func() {
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Fatal(err)
	}
	err = cmd.Start()
	if err != nil {
		log.Fatal(err)
	}
	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		if scanner.Text() == "HTTP server started" {
			break
		}
	}
})

var _ = AfterSuite(func() {
	err := cmd.Process.Kill()
	if err != nil {
		log.Fatal(err)
	}
})
