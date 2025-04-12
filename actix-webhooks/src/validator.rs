use crate::admission_review::AdmissionReview;
use actix_web::{Responder, post, web};
use tracing::info;

const EXPECTED_ANNOTATION: &str = "app.kubernetes.io/instance";

#[post("/validate")]
async fn validate(request: web::Json<AdmissionReview>) -> std::io::Result<impl Responder> {
    info!("Validating Pod");
    let review_request = request.request.clone().unwrap();
    let uid = review_request.uid;
    if let Some(annotations) = review_request.object.metadata.annotations {
        if annotations.contains_key(EXPECTED_ANNOTATION) {
            Ok(web::Json(AdmissionReview::response(uid.as_str(), true)))
        } else {
            Ok(web::Json(AdmissionReview::response_with_status(
                uid.as_str(),
                false,
                400,
                format!("Annotation {} not found", EXPECTED_ANNOTATION).as_str(),
            )))
        }
    } else {
        Ok(web::Json(AdmissionReview::response_with_status(
            uid.as_str(),
            false,
            400,
            "Annotations not found",
        )))
    }
}
