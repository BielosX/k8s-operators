mod k8s_client;

use axum::routing::get;
use axum::Router;
use std::env;
use tokio::select;
use tokio::time::{sleep, Duration};
use tracing::info;
use k8s_client::client::K8sClient;

async fn operator() {
    let client = K8sClient::new().await;
    loop {
        let response = client.get_exposed_apps().await;
        info!("Received: {}", response);
        sleep(Duration::from_secs(10)).await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port = env::var("PORT").unwrap_or("3000".to_string());
    let app = Router::new().route("/healthz", get(async || "OK"));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    select! {
        _ = tokio::spawn(operator()) => {}
        _ = axum::serve(listener, app) => {}
    }
}
