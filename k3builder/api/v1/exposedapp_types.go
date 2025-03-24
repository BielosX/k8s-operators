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

	Replicas int32  `json:"replicas"`
	Image    string `json:"image"`
	// +kubebuilder:validation:Enum=TCP;UDP
	Protocol      string `json:"protocol"`
	Port          int32  `json:"port"`
	ContainerPort int32  `json:"containerPort"`
	NodePort      *int32 `json:"nodePort,omitempty"`
	// +kubebuilder:validation:Enum=ClusterIP;NodePort
	ServiceType string `json:"serviceType,omitempty"`
}

// ExposedAppStatus defines the observed state of ExposedApp.
type ExposedAppStatus struct {
	// INSERT ADDITIONAL STATUS FIELD - define observed state of cluster
	// Important: Run "make" to regenerate code after modifying this file

	DeploymentName string `json:"deploymentName,omitempty"`
	ServiceName    string `json:"serviceName,omitempty"`
	LastUpdateBy   string `json:"lastUpdateBy,omitempty"`
	// https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md#typical-status-properties
	Conditions []metav1.Condition `json:"conditions,omitempty" patchStrategy:"merge" patchMergeKey:"type" protobuf:"bytes,1,rep,name=conditions"`
}

// +kubebuilder:object:root=true
// +kubebuilder:subresource:status
// +kubebuilder:printcolumn:name="Deployment",type="string",JSONPath=".status.deploymentName"
// +kubebuilder:printcolumn:name="Service",type="string",JSONPath=".status.serviceName"
// +kubebuilder:printcolumn:name="Last Update By",type="string",JSONPath=".status.lastUpdateBy"

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
