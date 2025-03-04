use axum::routing::get;
use axum::Router;
use std::env;
use reqwest::{Certificate, Client};
use tokio::select;
use tokio::time::{sleep, Duration};
use tracing::info;
use tokio::fs;

const SERVICE_ACCOUNT_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount";
const API_SERVER: &str = "https://kubernetes.default.svc";
const EXPOSED_APPS_LIST: &str = "/apis/stable.no-library.com/v1/exposedapps";

async fn get_token() -> Result<String, String> {
    let content = fs::read(format!("{}/token", SERVICE_ACCOUNT_PATH))
        .await.map_err(|err| err.to_string())?;
    String::from_utf8(content).map_err(|_err| { "Unable to load token".to_string() })
}

async fn get_client() -> Result<Client, String> {
    let cert = fs::read(format!("{}/ca.crt", SERVICE_ACCOUNT_PATH))
        .await.map_err(|err| err.to_string())?;
    let certificate = Certificate::from_pem(cert.as_slice()).map_err(|err| err.to_string())?;
    Client::builder().add_root_certificate(certificate).build().map_err(|err| err.to_string())
}

async fn operator() {
    let client = get_client().await.unwrap();
    loop {
        let token = get_token().await.unwrap();
        info!("Token fetched");
        let request = client.get(format!("{}{}", API_SERVER, EXPOSED_APPS_LIST))
            .header("Authorization", format!("Bearer {}", token))
            .build().unwrap();
        let response = client.execute(request).await.map_err(|err| err.to_string()).unwrap();
        info!("Received: {}", response.text().await.unwrap());
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
