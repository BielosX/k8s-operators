use crate::exposed_app_controller::run_controller;
use axum::Router;
use axum::routing::get;
use kube::{Client, Result};
use std::env;
use tokio::select;
use tracing::{error, info};

mod exposed_app;
mod exposed_app_controller;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting ExposedApp controller");
    let port = env::var("PORT").unwrap_or(String::from("8080"));
    let app = Router::new().route("/healthz", get(|| async { "OK" }));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    let serve = axum::serve(listener, app);
    let client = Client::try_default().await?;
    let controller = run_controller(client.clone());
    select! {
        val = serve => error!("serve exited with {:?}", val),
        _ = controller => error!("controller exited"),
    }
    Ok(())
}
