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

	It("Should fail with BadRequest when AdmissionReview Request not provided", func() {
		resp, err := http.Post("http://localhost:8080/validate",
			"application/json",
			strings.NewReader("{}"))
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusBadRequest))
	})

	It("Should fail with BadRequest when invalid AdmissionReview provided", func() {
		resp, err := http.Post("http://localhost:8080/validate",
			"application/json",
			strings.NewReader("[]"))
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusBadRequest))
	})

	It("Should accept Deployment with no annotations", func() {
		uid := "705ab4f5-6393-11e8-b7cc-42010a800002"
		resp, err := http.Post("http://localhost:8080/validate",
			"application/json",
			strings.NewReader(e2e.RequestWithUid(uid)))
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusOK))

		review := e2e.ParseBody(resp.Body)
		response := review["response"].(map[string]any)
		Expect(response["allowed"]).To(BeTrue())
		Expect(response["uid"]).To(Equal(uid))
	})

	It("Should reject Deployment with immutable annotation", func() {
		uid := "705ab4f5-6393-11e8-b7cc-42010a800002"
		annotations := map[string]string{"immutable": "true"}
		resp, err := http.Post("http://localhost:8080/validate",
			"application/json",
			strings.NewReader(e2e.RequestWithUidAndAnnotations(uid, annotations)))
		Expect(err).ToNot(HaveOccurred())
		Expect(resp.StatusCode).To(Equal(http.StatusOK))

		review := e2e.ParseBody(resp.Body)
		response := review["response"].(map[string]any)
		status := response["status"].(map[string]any)
		message := status["message"].(string)
		Expect(response["allowed"]).To(BeFalse())
		Expect(response["uid"]).To(Equal(uid))
		Expect(message).To(Equal("Deployment marked as Immutable, unable to Update. Delete first, then recreate"))
	})
})
