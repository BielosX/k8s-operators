use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub labels: Option<HashMap<String, String>>,
    pub annotations: Option<HashMap<String, String>>,
    pub resource_version: Option<String>,
    pub finalizers: Option<Vec<String>>,
    pub deletion_timestamp: Option<String>,
    pub name: Option<String>,
    pub namespace: Option<String>,
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
pub struct ExposedApp {
    pub spec: ExposedAppSpec,
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
pub struct Container {
    pub name: String,
    pub image: String,
}
