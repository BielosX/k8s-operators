package e2e_test

import (
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	"io"
	"net/http"
	"strings"
)

var _ = Describe("E2e", func() {
	It("Should pass health check", func() {
		resp, err := http.Get("http://localhost:8080/healthz")
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusOK))
	})

	It("Should return expected metrics", func() {
		resp, err := http.Get("http://localhost:8080/metrics")
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusOK))
		buf := new(strings.Builder)
		_, err = io.Copy(buf, resp.Body)
		Expect(err).ToNot(HaveOccurred())
		str := buf.String()
		Expect(str).To(ContainSubstring("webhook_conditions_allowed"))
		Expect(str).To(ContainSubstring("webhook_conditions_validate_internal_server_error_total"))
		Expect(str).To(ContainSubstring("webhook_conditions_denied"))
		Expect(str).To(ContainSubstring("webhook_conditions_validate_requests"))
	})
})
