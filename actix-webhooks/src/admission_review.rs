use serde::{Deserialize, Serialize};

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
pub struct AdmissionReviewRequest {
    pub uid: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdmissionReviewResponse {
    uid: String,
    allowed: bool,
}

impl AdmissionReview {
    pub fn response(uuid: &str, allowed: bool) -> Self {
        AdmissionReview {
            api_version: String::from("admission.k8s.io/v1"),
            kind: String::from("AdmissionReview"),
            request: None,
            response: Some(AdmissionReviewResponse {
                uid: String::from(uuid),
                allowed,
            }),
        }
    }
}
