/*
Copyright 2025.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package v1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// EDIT THIS FILE!  THIS IS SCAFFOLDING FOR YOU TO OWN!
// NOTE: json tags are required.  Any new fields you add must have json tags for the fields to be serialized.

// ExposedAppSpec defines the desired state of ExposedApp.
type ExposedAppSpec struct {
	// INSERT ADDITIONAL SPEC FIELDS - desired state of cluster
	// Important: Run "make" to regenerate code after modifying this file

	Replicas      int    `json:"replicas"`
	Image         string `json:"image"`
	Protocol      string `json:"protocol"`
	Port          int    `json:"port"`
	ContainerPort int    `json:"containerPort"`
	NodePort      int    `json:"nodePort"`
	ServiceType   string `json:"serviceType"`
}

// ExposedAppStatus defines the observed state of ExposedApp.
type ExposedAppStatus struct {
	// INSERT ADDITIONAL STATUS FIELD - define observed state of cluster
	// Important: Run "make" to regenerate code after modifying this file

	DeploymentName string `json:"deploymentName"`
	ServiceName    string `json:"serviceName"`
	LastUpdateBy   string `json:"lastUpdateBy"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status

// ExposedApp is the Schema for the exposedapps API.
type ExposedApp struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   ExposedAppSpec   `json:"spec,omitempty"`
	Status ExposedAppStatus `json:"status,omitempty"`
}

// +kubebuilder:object:root=true

// ExposedAppList contains a list of ExposedApp.
type ExposedAppList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []ExposedApp `json:"items"`
}

func init() {
	SchemeBuilder.Register(&ExposedApp{}, &ExposedAppList{})
}
