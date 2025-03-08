mod k8s_client;
mod k8s_types;
mod operator;

use axum::routing::get;
use axum::Router;
use operator::operator::handle_custom_resources;
use std::env;
use tokio::select;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port = env::var("PORT").unwrap_or("3000".to_string());
    let app = Router::new().route("/healthz", get(async || "OK"));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    select! {
        _ = tokio::spawn(handle_custom_resources()) => {}
        _ = axum::serve(listener, app) => {}
    }
}
