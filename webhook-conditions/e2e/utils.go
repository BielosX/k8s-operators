package e2e

import (
	"encoding/json"
	"fmt"
	"io"
	"strings"
)

func ToString(reader io.Reader) (string, error) {
	buf := new(strings.Builder)
	_, err := io.Copy(buf, reader)
	if err != nil {
		return "", err
	}
	return buf.String(), nil
}

func ParseBody(reader io.Reader) map[string]any {
	str, _ := ToString(reader)
	var review map[string]any
	_ = json.Unmarshal([]byte(str), &review)
	return review
}

func RequestWithUid(uid string) string {
	payload := `
		{
			"apiVersion": "admission.k8s.io/v1",
			"kind": "AdmissionReview",
			"request": {
				"uid": "%s",
				"oldObject": {
					"apiVersion": "apps/v1",
					"kind": "Deployment",
					"metadata": {
						"name": "nginx-deployment"
					}
				}
			}
		}`
	return fmt.Sprintf(payload, uid)
}

func RequestWithUidAndAnnotations(uid string, annotations map[string]string) string {
	marshalled, _ := json.Marshal(annotations)
	payload := `
		{
			"apiVersion": "admission.k8s.io/v1",
			"kind": "AdmissionReview",
			"request": {
				"uid": "%s",
				"oldObject": {
					"apiVersion": "apps/v1",
					"kind": "Deployment",
					"metadata": {
						"name": "nginx-deployment",
						"annotations": %s		
					}
				}
			}
		}`
	return fmt.Sprintf(payload, uid, marshalled)
}
