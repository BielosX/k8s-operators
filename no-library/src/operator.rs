pub mod operator {
    use crate::k8s_client::client::K8sClient;
    use crate::leader_election::LeaderElector;
    use crate::reconciler::Reconciler;
    use futures::{pin_mut, StreamExt};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc::{Receiver, Sender};
    use tokio::time::sleep;
    use tracing::info;

    pub async fn elect_leader(
        pod_id: String,
        is_leader_elected: Arc<AtomicBool>,
        is_leader_sender: Sender<()>,
    ) {
        let mut leader_elector = LeaderElector::new(
            K8sClient::new().await,
            pod_id.as_str(),
            is_leader_elected,
            is_leader_sender,
        );
        leader_elector.elect_leader().await;
    }

    pub async fn handle_custom_resources(mut receiver: Receiver<()>) {
        receiver.recv().await;
        info!("Starting doing operator stuff");
        let mut client = K8sClient::new().await;
        let mut reconciler = Reconciler::new(client.clone());
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
