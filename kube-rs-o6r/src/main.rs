use crate::exposed_app::ExposedApp;
use crate::exposed_app_controller::{Data, error_policy, reconcile};
use axum::Router;
use axum::routing::get;
use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use kube::runtime::Controller;
use kube::runtime::events::{Recorder, Reporter};
use kube::runtime::watcher::Config;
use kube::{Api, Client, Result};
use std::env;
use std::sync::Arc;
use tokio::select;
use tracing::{error, info, warn};

mod exposed_app;
mod exposed_app_controller;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting ExposedApp controller");
    let client = Client::try_default().await?;
    let exposed_apps: Api<ExposedApp> = Api::all(client.clone());
    let deployments: Api<Deployment> = Api::all(client.clone());
    let services: Api<Service> = Api::all(client.clone());

    let port = env::var("PORT").unwrap_or(String::from("8080"));
    let app = Router::new().route("/healthz", get(|| async { "OK" }));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    let serve = axum::serve(listener, app);
    let reporter = Reporter {
        controller: String::from("exposed-app-controller"),
        instance: None,
    };
    let recorder = Recorder::new(client.clone(), reporter);
    let data = Arc::new(Data::new(client.clone(), recorder));
    let controller = Controller::new(exposed_apps, Config::default())
        .owns(deployments, Config::default())
        .owns(services, Config::default())
        .run(reconcile, error_policy, Arc::clone(&data))
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => warn!("reconcile failed: {:?}", e),
            }
        });
    select! {
        val = serve => error!("serve exited with {:?}", val),
        val = controller => error!("controller exited with {:?}", val),
    }
    Ok(())
}
