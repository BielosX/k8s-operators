mod admission_review;
mod modifier;
mod validator;

use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, Responder, get};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

#[get("/healthz")]
async fn health() -> impl Responder {
    "OK"
}

const CERT_DIR: &str = "/etc/ssl/private";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    let port = std::env::var("PORT").unwrap_or(String::from("8080"));
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls_server())?;
    builder.set_private_key_file(format!("{}/tls.key", CERT_DIR), SslFiletype::PEM)?;
    builder.set_certificate_file(format!("{}/tls.crt", CERT_DIR), SslFiletype::PEM)?;
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(health)
            .service(validator::validate)
            .service(modifier::modify)
    })
    .bind_openssl(format!("0.0.0.0:{}", port), builder)?
    .run()
    .await
}
