pub mod operator {
    use crate::k8s_client::client::{K8sClient, K8sClientError};
    use crate::k8s_types::ExposedApp;
    use futures::{pin_mut, StreamExt};
    use std::time::Duration;
    use tokio::time::sleep;
    use tracing::info;

    const FINALIZER_NAME: &str = "exposedapps.stable.no-library.com/finalizer";

    pub async fn handle_custom_resources() {
        let client = K8sClient::new().await;
        loop {
            let exposed_apps = client.get_exposed_apps().await;
            let stream = client
                .watch_exposed_apps(exposed_apps.metadata.resource_version.unwrap().as_str())
                .await;
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                info!(
                    "Event: {:?}, Resource: {:?}",
                    event.event_type, event.object.metadata.name
                );
                reconcile(&client, &event.object).await;
            }
            sleep(Duration::from_secs(10)).await;
        }
    }

    async fn reconcile(client: &K8sClient, resource: &ExposedApp) {
        let mut resource_copy: ExposedApp = resource.clone();
        match &resource.metadata.finalizers {
            None => {
                info!("Adding finalizer");
                resource_copy.metadata.finalizers = Some(vec![FINALIZER_NAME.to_string()]);
                client.update_exposed_apps(resource_copy).await.unwrap();
            }
            Some(finalizers) => {
                if resource.metadata.deletion_timestamp.is_some() {
                    info!("Removing finalizer");
                    if finalizers.len() == 1 {
                        info!("{} is the only one finalizer", FINALIZER_NAME);
                        resource_copy.metadata.finalizers = None;
                    } else {
                        info!("More than one finalizer found, removing {} only", FINALIZER_NAME);
                        let new_finalizers: Vec<String> = finalizers
                            .iter()
                            .filter(|&i| i != FINALIZER_NAME)
                            .map(Clone::clone)
                            .collect();
                        resource_copy.metadata.finalizers = Some(new_finalizers);
                    }
                    match client.update_exposed_apps(resource_copy).await {
                        Ok(_) => {}
                        Err(K8sClientError::NotFound) => {
                            info!("Nothing to update, it's fine");
                        }
                    }
                }
            }
        }
    }
}
