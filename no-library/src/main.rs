use axum::routing::get;
use axum::Router;
use tokio::select;
use tokio::time::{sleep, Duration};
use tracing::info;

async fn get_ip() -> reqwest::Result<String> {
    reqwest::get("https://api.ipify.org").await?.text().await
}

async fn operator() {
    loop {
        let ip = get_ip().await.unwrap();
        info!("Public IP: {}", ip);
        sleep(Duration::from_secs(10)).await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new().route("/healthz", get(async || "OK"));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    select! {
        _ = tokio::spawn(operator()) => {}
        _ = axum::serve(listener, app) => {}
    }
}
