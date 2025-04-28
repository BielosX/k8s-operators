package reconciler

import metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

type MyDeploymentSpec struct {
	Replicas string `json:"replicas"`
	Image    string `json:"image"`
}

type MyDeploymentStatus struct {
	DeploymentName string `json:"deploymentName,omitempty"`
}

type MyDeployment struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   MyDeploymentSpec   `json:"spec,omitempty"`
	Status MyDeploymentStatus `json:"status,omitempty"`
}
