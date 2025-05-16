use k8s_openapi::serde::{Deserialize, Serialize};
use kube::CustomResource;
use schemars::JsonSchema;

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema, Hash)]
#[kube(
    group = "stable.kube-rs-o6r.com",
    version = "v1",
    kind = "ExposedApp",
    namespaced
)]
#[kube(status = "ExposedAppStatus")]
#[serde(rename_all = "camelCase")]
#[kube(
    printcolumn = r#"{"name":"DeploymentName", "type":"string", "description":"Deployment Name", "jsonPath":".status.deploymentName"}"#
)]
#[kube(
    printcolumn = r#"{"name":"ServiceName", "type":"string", "description":"Service Name", "jsonPath":".status.serviceName"}"#
)]
pub struct ExposedAppSpec {
    pub image: String,
    pub replicas: i32,
    #[schemars(range(min = 1, max = 65535))]
    pub port: i32,
    pub protocol: String,
    #[schemars(range(min = 1, max = 65535))]
    pub container_port: i32,
    pub node_port: Option<i32>,
    pub service_type: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExposedAppStatus {
    // When Merge Patch is used, don't remove the field when None used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
}
