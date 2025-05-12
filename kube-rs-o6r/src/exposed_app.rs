use k8s_openapi::serde::{Deserialize, Serialize};
use kube::CustomResource;
use schemars::JsonSchema;

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(group = "stable.kube-rs-o6r.com", version = "v1", kind = "ExposedApp", namespaced)]
#[kube(status = "ExposedAppStatus")]
#[serde(rename_all = "camelCase")]
pub struct ExposedAppSpec {
    pub image: String,
    pub replicas: u32,
    #[schemars(range(min = 1, max = 65535))]
    pub port: u16,
    pub protocol: String,
    #[schemars(range(min = 1, max = 65535))]
    pub container_port: Option<u16>,
    pub node_port: Option<u16>,
    pub service_type: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExposedAppStatus {
    pub deployment_name: String,
    pub service_name: String,
}
