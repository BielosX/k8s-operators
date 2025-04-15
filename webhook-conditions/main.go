package main

import (
	"encoding/json"
	"fmt"
	admissionv1 "k8s.io/api/admission/v1"
	appsv1 "k8s.io/api/apps/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"log/slog"
	"net"
	"net/http"
	"os"
	"strconv"
)

type statusAwareResponseWriter struct {
	http.ResponseWriter
	statusCode int
}

func (writer *statusAwareResponseWriter) WriteHeader(statusCode int) {
	writer.statusCode = statusCode
	writer.WriteHeader(statusCode)
}

func handleFunc(pattern string, handler func(http.ResponseWriter, *http.Request)) {
	http.HandleFunc(pattern, func(writer http.ResponseWriter, request *http.Request) {
		statusAwareWriter := statusAwareResponseWriter{writer, http.StatusOK}
		handler(&statusAwareWriter, request)
		slog.Info(fmt.Sprintf("Request %s %s responded %d",
			request.Method, request.URL, statusAwareWriter.statusCode))
	})
}

func validate(w http.ResponseWriter, r *http.Request) {
	review := admissionv1.AdmissionReview{}
	err := json.NewDecoder(r.Body).Decode(&review)
	if err != nil {
		slog.Error("Unable to parse AdmissionReview", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	slog.Info("Received AdmissionRequest", "uid", review.Request.UID)
	oldDeployment := &appsv1.Deployment{}
	err = json.Unmarshal(review.Request.OldObject.Raw, oldDeployment)
	if err != nil {
		slog.Error("Unable to parse Old Deployment", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	review.Response = &admissionv1.AdmissionResponse{
		Allowed: true,
		UID:     review.Request.UID,
	}
	if oldDeployment.Annotations != nil {
		value, ok := oldDeployment.Annotations["immutable"]
		if ok && value == "true" {
			slog.Info("Deployment marked as immutable")
			review.Response.Allowed = false
			review.Response.Result = &metav1.Status{
				Code:    http.StatusBadRequest,
				Message: "Deployment marked as Immutable, unable to Update. Delete first, then recreate",
			}
		}
	}
	payload, err := json.Marshal(review)
	if err != nil {
		slog.Error("Unable to serialize AdmissionReview", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}
	_, err = w.Write(payload)
	if err != nil {
		slog.Error("Unable to send response", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
	}
}

func health(writer http.ResponseWriter, _ *http.Request) {
	_, err := writer.Write([]byte("OK"))
	if err != nil {
		slog.Error("Unable to send response", "error", err)
	}
}

func main() {
	port, err := strconv.ParseUint(os.Getenv("PORT"), 10, 16)
	if err != nil {
		port = 8080
	}
	certFile := os.Getenv("CERT_FILE")
	keyFile := os.Getenv("KEY_FILE")

	handleFunc("/validate", validate)
	handleFunc("/healthz", health)

	listener, err := net.Listen("tcp", fmt.Sprintf("0.0.0.0:%d", port))
	if err == nil {
		slog.Info(fmt.Sprintf("Listening on port %d", port))
	} else {
		slog.Error(fmt.Sprintf("Unable to listen on port %d", port), "error", err)
	}
	if len(certFile) == 0 || len(keyFile) == 0 {
		err = http.Serve(listener, nil)
		if err != nil {
			slog.Error("Unable to serve HTTP traffic", "error", err)
		}
	} else {
		slog.Info(fmt.Sprintf("Using CertFile %s, KeyFile %s", certFile, keyFile))
		err = http.ServeTLS(listener, nil, certFile, keyFile)
		if err != nil {
			slog.Error("Unable to serve HTTPS traffic", "error", err)
		}
	}
}
