use crate::k8s_client::client::{K8sClient, K8sClientError};
use crate::k8s_types::ExposedApp;
use tracing::{error, info};

const FINALIZER_NAME: &str = "exposedapps.stable.no-library.com/finalizer";

pub struct Reconciler<'a> {
    client: &'a K8sClient,
}

impl<'a> Reconciler<'a> {
    pub fn new(client: &'a K8sClient) -> Self {
        Reconciler { client }
    }

    async fn clean_up(&self, resource: &ExposedApp) {}

    async fn handle_finalizer(&self, resource: &ExposedApp) -> bool {
        let mut resource_copy: ExposedApp = resource.clone();
        match &resource.metadata.finalizers {
            None => {
                info!("Adding finalizer");
                resource_copy.metadata.finalizers = Some(vec![FINALIZER_NAME.to_string()]);
                self.client
                    .update_exposed_apps(resource_copy)
                    .await
                    .unwrap();
                false
            }
            Some(finalizers) => {
                if resource.metadata.deletion_timestamp.is_some() {
                    info!("Removing finalizer");
                    if finalizers.len() == 1 {
                        info!("{} is the only one finalizer", FINALIZER_NAME);
                        resource_copy.metadata.finalizers = None;
                    } else {
                        info!(
                            "More than one finalizer found, removing {} only",
                            FINALIZER_NAME
                        );
                        let new_finalizers: Vec<String> = finalizers
                            .iter()
                            .filter(|&i| i != FINALIZER_NAME)
                            .map(Clone::clone)
                            .collect();
                        resource_copy.metadata.finalizers = Some(new_finalizers);
                    }
                    self.clean_up(&resource_copy).await;
                    match self.client.update_exposed_apps(resource_copy).await {
                        Ok(_) => {}
                        Err(K8sClientError::NotFound) => {
                            info!("Nothing to update, it's fine");
                        }
                        Err(K8sClientError::Error) => {
                            error!("Something wrong happened");
                        }
                    }
                    true
                } else {
                    false
                }
            }
        }
    }

    pub async fn reconcile(&self, resource: &ExposedApp) {
        let finalized = self.handle_finalizer(resource).await;
        if !finalized {
            info!("Synchronizing resource");
        }
    }
}
