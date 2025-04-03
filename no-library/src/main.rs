mod cache;
mod k8s_client;
mod k8s_types;
mod leader_election;
mod offset_date_time_parser;
mod operator;
mod reconciler;

use crate::operator::operator::elect_leader;
use axum::routing::get;
use axum::Router;
use operator::operator::handle_owned_resources;
use std::env;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port = env::var("PORT").unwrap_or("3000".to_string());
    let pod_name = env::var("POD_NAME").expect("Pod name expected");
    let app = Router::new().route("/healthz", get(async || "OK"));
    let (sender, receiver): (Sender<()>, Receiver<()>) = mpsc::channel(1);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    select! {
        _ = tokio::spawn(elect_leader(pod_name.clone(), sender)) => {}
        _ = tokio::spawn(handle_owned_resources(receiver, pod_name)) => {}
        _ = axum::serve(listener, app) => {}
    }
}
