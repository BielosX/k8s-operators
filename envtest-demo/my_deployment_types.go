package reconciler

import (
	"github.com/jinzhu/copier"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"sigs.k8s.io/controller-runtime/pkg/scheme"
)

type MyDeploymentSpec struct {
	Replicas int32  `json:"replicas"`
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

func (m MyDeployment) DeepCopyObject() runtime.Object {
	newCopy := &MyDeployment{}
	err := copier.Copy(newCopy, &m)
	if err != nil {
		return nil
	}
	return newCopy
}

var (
	GroupVersion  = schema.GroupVersion{Group: "stable.demo.com", Version: "v1"}
	SchemeBuilder = &scheme.Builder{GroupVersion: GroupVersion}
)
