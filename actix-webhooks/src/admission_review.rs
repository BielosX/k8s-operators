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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdmissionReviewResponseStatus {
    code: u32,
    message: String,
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
            }),
            ..AdmissionReview::default()
        }
    }
}
