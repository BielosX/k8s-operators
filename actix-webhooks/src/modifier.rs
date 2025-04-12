use crate::admission_review::{AdmissionReview, JsonPatchEntry, JsonPatchOperation};
use actix_web::{Responder, post, web};
use tracing::info;

// https://www.rfc-editor.org/rfc/rfc6901#section-3
const ANNOTATION_KEY: &str = "kubernetes.io~1description";

#[post("/modify")]
async fn modify(request: web::Json<AdmissionReview>) -> std::io::Result<impl Responder> {
    info!("Modifying Pod");
    let review_request = request.request.clone().unwrap();
    let uid = review_request.uid;
    Ok(web::Json(AdmissionReview::response_with_patch(
        uid.as_str(),
        vec![JsonPatchEntry {
            op: JsonPatchOperation::Add,
            path: format!("/metadata/annotations/{}", ANNOTATION_KEY),
            value: String::from("Modified by Actix Webhooks"),
        }],
    )))
}
