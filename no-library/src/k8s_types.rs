use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OwnerReference {
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub uid: String,
    pub block_owner_deletion: bool,
    pub controller: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ObjectReference {
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub uid: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum EventType {
    Normal,
    Warning,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_time: String,
    pub action: String,
    pub note: Option<String>,
    pub reason: String,
    pub regarding: Option<ObjectReference>,
    pub related: Option<ObjectReference>,
    pub reporting_controller: String,
    pub reporting_instance: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ManageFields {
    pub manager: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub labels: Option<HashMap<String, String>>,
    pub annotations: Option<HashMap<String, String>>,
    pub resource_version: Option<String>,
    pub finalizers: Option<Vec<String>>,
    pub deletion_timestamp: Option<String>,
    pub name: Option<String>,
    pub uid: Option<String>,
    pub namespace: Option<String>,
    pub owner_references: Option<Vec<OwnerReference>>,
    pub generation: Option<u64>,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            labels: None,
            annotations: None,
            resource_version: None,
            finalizers: None,
            deletion_timestamp: None,
            name: None,
            namespace: None,
            owner_references: None,
            uid: None,
            generation: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum WatchEventType {
    Added,
    Modified,
    Deleted,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExposedAppSpec {
    pub replicas: u32,
    pub container_port: u32,
    pub image: String,
    pub port: u32,
    pub protocol: String,
    pub node_port: Option<u32>,
    pub service_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct K8sObject<T> {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    #[serde(flatten)]
    pub object: T,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct K8sListObject<T> {
    pub metadata: Metadata,
    #[serde(flatten)]
    pub object: T,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExposedAppStatus {
    pub deployment_name: String,
    pub service_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExposedApp {
    pub spec: ExposedAppSpec,
    pub status: Option<ExposedAppStatus>,
}

#[derive(Serialize, Deserialize)]
pub struct Watch<T> {
    #[serde(rename = "type")]
    pub event_type: WatchEventType,
    pub object: T,
}

#[derive(Serialize, Deserialize)]
pub struct List<T> {
    pub items: Vec<T>,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
pub struct Deployment {
    pub spec: DeploymentSpec,
}

#[derive(Serialize, Deserialize)]
pub struct DeploymentSpec {
    pub replicas: u32,
    pub template: PodTemplate,
    pub selector: Selector,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selector {
    pub match_labels: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct PodTemplate {
    pub metadata: Metadata,
    pub spec: PodSpec,
}

#[derive(Serialize, Deserialize)]
pub struct PodSpec {
    pub containers: Vec<Container>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPort {
    pub container_port: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Container {
    pub name: String,
    pub image: String,
    pub ports: Option<Vec<ContainerPort>>,
}

#[derive(Serialize, Deserialize)]
pub struct Service {
    pub spec: ServiceSpec,
}

#[derive(Serialize, Deserialize)]
pub struct ServiceSpec {
    #[serde(rename = "type")]
    pub service_type: Option<String>,
    pub selector: Option<HashMap<String, String>>,
    pub ports: Vec<ServicePort>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicePort {
    pub protocol: String,
    pub port: u32,
    pub target_port: u32,
    pub node_port: Option<u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseSpec {
    pub lease_duration_seconds: u32,
    pub holder_identity: Option<String>,
    pub acquire_time: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Lease {
    pub spec: LeaseSpec,
}

impl<'a, T> Into<ObjectReference> for &'a K8sObject<T> {
    fn into(self) -> ObjectReference {
        ObjectReference {
            api_version: self.api_version.clone(),
            kind: self.kind.clone(),
            name: self.metadata.name.clone().unwrap(),
            namespace: self.metadata.namespace.clone().unwrap(),
            uid: self.metadata.uid.clone().unwrap(),
        }
    }
}

impl<T> Into<ObjectReference> for K8sObject<T> {
    fn into(self) -> ObjectReference {
        ObjectReference {
            api_version: self.api_version,
            kind: self.kind,
            name: self.metadata.name.unwrap(),
            namespace: self.metadata.namespace.unwrap(),
            uid: self.metadata.uid.unwrap(),
        }
    }
}

pub trait MetadataAware {
    fn metadata(&self) -> Metadata;
}

impl<T> MetadataAware for K8sObject<T> {
    fn metadata(&self) -> Metadata {
        self.metadata.clone()
    }
}

impl<T> MetadataAware for K8sListObject<T> {
    fn metadata(&self) -> Metadata {
        self.metadata.clone()
    }
}
