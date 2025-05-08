package main

import (
	"encoding/json"
	"fmt"
	"github.com/gorilla/handlers"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	admissionv1 "k8s.io/api/admission/v1"
	appsv1 "k8s.io/api/apps/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"log/slog"
	"net"
	"net/http"
	"os"
	"strconv"
)

func handleFunc(pattern string, handler func(http.ResponseWriter, *http.Request)) {
	http.Handle(pattern, handlers.LoggingHandler(os.Stdout, http.HandlerFunc(handler)))
}

func handle(pattern string, handler http.Handler) {
	http.Handle(pattern, handlers.LoggingHandler(os.Stdout, handler))
}

func validate(request *admissionv1.AdmissionRequest) (*admissionv1.AdmissionResponse, error) {
	slog.Info("Received AdmissionRequest", "uid", request.UID)
	oldDeployment := &appsv1.Deployment{}
	err := json.Unmarshal(request.OldObject.Raw, oldDeployment)
	if err != nil {
		slog.Error("Unable to parse Old Deployment", "error", err)
		return nil, err
	}
	response := &admissionv1.AdmissionResponse{
		Allowed: true,
		UID:     request.UID,
	}
	if oldDeployment.Annotations != nil {
		value, ok := oldDeployment.Annotations["immutable"]
		if ok && value == "true" {
			slog.Info("Deployment marked as immutable")
			response.Allowed = false
			response.Result = &metav1.Status{
				Code:    http.StatusBadRequest,
				Message: "Deployment marked as Immutable, unable to Update. Delete first, then recreate",
			}
		}
	}
	return response, nil
}

func validateHandler(w http.ResponseWriter, r *http.Request) {
	review := admissionv1.AdmissionReview{}
	err := json.NewDecoder(r.Body).Decode(&review)
	if err != nil {
		slog.Error("Unable to parse AdmissionReview", "error", err)
		w.WriteHeader(http.StatusBadRequest)
		serverErrors.Inc()
		return
	}
	if review.Request == nil {
		slog.Error("Request field not found in AdmissionReview")
		w.WriteHeader(http.StatusBadRequest)
		serverErrors.Inc()
		return
	}
	requests.Inc()
	response, err := validate(review.Request)
	if err != nil {
		slog.Error("Validate failed with error", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		serverErrors.Inc()
		return
	}
	review.Request = nil
	review.Response = response
	if response.Allowed {
		allowed.Inc()
	} else {
		denied.Inc()
	}
	payload, err := json.Marshal(review)
	if err != nil {
		slog.Error("Unable to serialize AdmissionReview", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		serverErrors.Inc()
		return
	}
	_, err = w.Write(payload)
	if err != nil {
		slog.Error("Unable to send response", "error", err)
		w.WriteHeader(http.StatusInternalServerError)
		serverErrors.Inc()
	}
}

func health(writer http.ResponseWriter, _ *http.Request) {
	_, err := writer.Write([]byte("OK"))
	if err != nil {
		slog.Error("Unable to send response", "error", err)
	}
}

var (
	serverErrors = promauto.NewCounter(prometheus.CounterOpts{
		Name: "webhook_conditions_validate_internal_server_error_total",
		Help: "The total number of 5xx returned by /validate",
	})
	allowed = promauto.NewCounter(prometheus.CounterOpts{
		Name: "webhook_conditions_allowed",
		Help: "The total number of allowed AdmissionRequests",
	})
	denied = promauto.NewCounter(prometheus.CounterOpts{
		Name: "webhook_conditions_denied",
		Help: "The total number of denied AdmissionRequests",
	})
	requests = promauto.NewCounter(prometheus.CounterOpts{
		Name: "webhook_conditions_validate_requests",
		Help: "The total number of all successfully parsed AdmissionRequests",
	})
)

func main() {
	port, err := strconv.ParseUint(os.Getenv("PORT"), 10, 16)
	if err != nil {
		port = 8080
	}
	certFile := os.Getenv("CERT_FILE")
	keyFile := os.Getenv("KEY_FILE")

	handleFunc("/validate", validateHandler)
	handleFunc("/healthz", health)
	handle("/metrics", promhttp.Handler())

	listener, err := net.Listen("tcp", fmt.Sprintf("0.0.0.0:%d", port))
	if err == nil {
		slog.Info(fmt.Sprintf("Listening on port %d", port))
	} else {
		slog.Error(fmt.Sprintf("Unable to listen on port %d", port), "error", err)
		os.Exit(1)
	}
	if len(certFile) == 0 || len(keyFile) == 0 {
		err = http.Serve(listener, nil)
		if err != nil {
			slog.Error("Unable to serve HTTP traffic", "error", err)
			os.Exit(1)
		}
		slog.Info("HTTP server started")
	} else {
		slog.Info(fmt.Sprintf("Using CertFile %s, KeyFile %s", certFile, keyFile))
		err = http.ServeTLS(listener, nil, certFile, keyFile)
		if err != nil {
			slog.Error("Unable to serve HTTPS traffic", "error", err)
			os.Exit(1)
		}
		slog.Info("HTTPS server started")
	}
}
