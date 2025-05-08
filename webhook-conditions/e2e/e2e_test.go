package e2e_test

import (
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	"net/http"
	"strings"
	"validator/immutable/e2e"
)

var _ = Describe("E2e", func() {
	It("Should return expected metrics", func() {
		resp, err := http.Get("http://localhost:8080/metrics")
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusOK))

		str, err := e2e.ToString(resp.Body)
		Expect(err).ToNot(HaveOccurred())
		Expect(str).To(ContainSubstring("webhook_conditions_allowed"))
		Expect(str).To(ContainSubstring("webhook_conditions_validate_internal_server_error_total"))
		Expect(str).To(ContainSubstring("webhook_conditions_denied"))
		Expect(str).To(ContainSubstring("webhook_conditions_validate_requests"))
	})

	It("Should fail with BadRequest when valid AdmissionReview not provided", func() {
		resp, err := http.Post("http://localhost:8080/validate",
			"application/json",
			strings.NewReader("{}"))
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusBadRequest))
	})
})
