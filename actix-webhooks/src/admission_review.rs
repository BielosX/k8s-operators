use base64::Engine;
use base64::engine::GeneralPurpose;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionReview {
    api_version: String,
    kind: String,
    pub request: Option<AdmissionReviewRequest>,
    response: Option<AdmissionReviewResponse>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pod {
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionReviewRequest {
    pub uid: String,
    pub object: Pod,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdmissionReviewResponse {
    uid: String,
    allowed: bool,
    status: Option<AdmissionReviewResponseStatus>,
    patch: Option<String>,
    patch_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdmissionReviewResponseStatus {
    code: u32,
    message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JsonPatchOperation {
    Add,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPatchEntry {
    pub op: JsonPatchOperation,
    pub path: String,
    pub value: String,
}

impl Default for AdmissionReview {
    fn default() -> Self {
        AdmissionReview {
            api_version: String::from("admission.k8s.io/v1"),
            kind: String::from("AdmissionReview"),
            request: None,
            response: None,
        }
    }
}

const BASE64_ENGINE: GeneralPurpose = base64::engine::general_purpose::STANDARD;

impl AdmissionReview {
    pub fn response_with_status(uuid: &str, allowed: bool, code: u32, message: &str) -> Self {
        AdmissionReview {
            response: Some(AdmissionReviewResponse {
                uid: String::from(uuid),
                allowed,
                status: Some(AdmissionReviewResponseStatus {
                    code,
                    message: String::from(message),
                }),
                patch: None,
                patch_type: None,
            }),
            ..AdmissionReview::default()
        }
    }

    pub fn response_with_patch(uuid: &str, operations: Vec<JsonPatchEntry>) -> Self {
        let encoded = BASE64_ENGINE.encode(serde_json::to_string(&operations).unwrap());
        AdmissionReview {
            response: Some(AdmissionReviewResponse {
                uid: String::from(uuid),
                allowed: true,
                status: None,
                patch: Some(encoded),
                patch_type: Some(String::from("JSONPatch")),
            }),
            ..AdmissionReview::default()
        }
    }

    pub fn response(uuid: &str, allowed: bool) -> Self {
        AdmissionReview {
            response: Some(AdmissionReviewResponse {
                uid: String::from(uuid),
                allowed,
                status: None,
                patch: None,
                patch_type: None,
            }),
            ..AdmissionReview::default()
        }
    }
}
