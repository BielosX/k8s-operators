use crate::exposed_app::ExposedApp;
use kube::runtime::controller::Action;
use kube::{Client, Error};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[derive(Clone)]
pub struct Data {
    client: Client,
}

impl Data {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

pub async fn reconcile(object: Arc<ExposedApp>, _ctx: Arc<Data>) -> Result<Action, Error> {
    info!(
        "Reconciling {} {}",
        object.metadata.name.clone().unwrap(),
        object.metadata.namespace.clone().unwrap()
    );
    Ok(Action::await_change())
}

pub fn error_policy(_object: Arc<ExposedApp>, _err: &Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(Duration::from_secs(5))
}
