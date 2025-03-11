pub mod operator {
    use crate::k8s_client::client::K8sClient;
    use crate::reconciler::Reconciler;
    use futures::{pin_mut, StreamExt};
    use std::time::Duration;
    use tokio::time::sleep;
    use tracing::info;

    pub async fn handle_custom_resources() {
        let client = K8sClient::new().await;
        let reconciler = Reconciler::new(&client);
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
                reconciler.reconcile(&event.object).await;
            }
            sleep(Duration::from_secs(10)).await;
        }
    }
}
