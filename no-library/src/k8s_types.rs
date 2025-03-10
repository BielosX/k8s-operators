use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub resource_version: Option<String>,
    pub finalizers: Option<Vec<String>>,
    pub deletion_timestamp: Option<String>,
    pub name: Option<String>,
    pub namespace: Option<String>,
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
    pub container_port: u32,
    pub image: String,
    pub port: u32,
    pub protocol: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExposedApp {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
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
