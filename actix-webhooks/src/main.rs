use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, Responder, get};

#[get("/healthz")]
async fn health() -> impl Responder {
    "OK"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    let port = std::env::var("PORT").unwrap_or(String::from("8080"));
    HttpServer::new(|| App::new().wrap(Logger::default()).service(health))
        .bind(format!("0.0.0.0:{}", port))?
        .run()
        .await
}
