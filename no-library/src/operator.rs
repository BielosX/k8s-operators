pub mod operator {
    use std::collections::HashMap;
    use crate::k8s_client::client::{K8sClient, K8sClientError};
    use crate::k8s_types::{
        Deployment, ExposedApp, K8sListObject, K8sObject, List, Service,
    };
    use crate::leader_election::LeaderElector;
    use crate::reconciler::Reconciler;
    use futures::{pin_mut, StreamExt};
    use serde::de::DeserializeOwned;
    use std::ops::Mul;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::select;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::{Receiver, Sender};
    use tokio::time::sleep;
    use tokio_util::time::DelayQueue;
    use tracing::{error, info};

    const ALL_DEPLOYMENTS_LIST: &str = "apis/apps/v1/deployments";
    const ALL_SERVICES_LIST: &str = "api/v1/services";

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

    pub async fn handle_owned_resources(mut receiver: Receiver<()>) {
        receiver.recv().await;
        info!("Started doing operator stuff");
        let (app_sender, app_receiver): (
            Sender<K8sObject<ExposedApp>>,
            Receiver<K8sObject<ExposedApp>>,
        ) = mpsc::channel(64);
        select! {
            _ = tokio::spawn(handle_reconcile_requests(app_receiver)) => {}
            _ = tokio::spawn(handle_exposed_apps(app_sender.clone())) => {}
            _ = tokio::spawn(handle_owned_update::<Deployment>(app_sender.clone(), ALL_DEPLOYMENTS_LIST)) => {}
            _ = tokio::spawn(handle_owned_update::<Service>(app_sender.clone(), ALL_SERVICES_LIST)) => {}
        }
    }

    struct QueueEntry {
        entry: K8sObject<ExposedApp>,
        delay: Duration,
    }

    impl QueueEntry {
        pub fn new(entry: K8sObject<ExposedApp>, delay: Duration) -> Self {
            QueueEntry { entry, delay }
        }
    }

    #[derive(Eq, PartialEq, Hash)]
    struct NamespacedName {
        pub namespace: String,
        pub name: String,
    }

    async fn handle_exposed_apps(sender: Sender<K8sObject<ExposedApp>>) {
        let mut client = K8sClient::new().await;
        loop {
            info!("Watching ExposedApp");
            let get_result = client.get_exposed_apps().await;
            let resource_version = get_result.metadata.resource_version.clone().unwrap();
            for item in get_result.items {
                sender.send(item).await.unwrap();
            }
            let stream = client.watch_exposed_apps(resource_version.as_str()).await;
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                info!("Received ExposedApp {:?} event", event.event_type);
                sender
                    .send(K8sObject {
                        api_version: String::from("stable.no-library.com/v1"),
                        kind: String::from("ExposedApp"),
                        metadata: event.object.metadata.clone(),
                        object: event.object.object.clone(),
                    })
                    .await
                    .unwrap();
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn handle_owned_update<T: DeserializeOwned>(
        sender: Sender<K8sObject<ExposedApp>>,
        uri: &str,
    ) {
        let mut client = K8sClient::new().await;
        loop {
            info!("Watching URI {}", uri);
            let get_result: List<K8sListObject<T>> = client.get_all(uri).await.unwrap();
            let resource_version = get_result.metadata.resource_version.clone().unwrap();
            let stream = client.watch::<T>(uri, resource_version.as_str()).await;
            pin_mut!(stream);
            while let Some(event) = stream.next().await {
                if let Some(references) = event.object.metadata.owner_references {
                    if let Some(exposed_app_ref) =
                        references.iter().find(|&item| item.kind == "ExposedApp")
                    {
                        let owner_name = exposed_app_ref.name.clone();
                        let object_name = event.object.metadata.name.unwrap();
                        let namespace = event.object.metadata.namespace.unwrap();
                        info!(
                            "Found ExposedApp owner {} for object {}",
                            owner_name, object_name
                        );
                        match client
                            .get_exposed_app(owner_name.as_str(), namespace.as_str())
                            .await
                        {
                            Ok(app) => {
                                sender.send(app).await.unwrap();
                            }
                            Err(K8sClientError::NotFound) => {
                                info!(
                                    "ExposedApp {} not found, probably already deleted",
                                    owner_name
                                );
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    fn handle_reconcile_error(delay: &mut Duration,
                              queue: &mut DelayQueue<QueueEntry>, exposed_app: &K8sObject<ExposedApp>,
                              err: String) {
        error!(err);
        if delay.is_zero() {
            *delay = Duration::from_secs(2);
        } else {
            if *delay < Duration::from_secs(128) {
                *delay = delay.mul(2);
            }
        }
        queue.insert(QueueEntry::new(exposed_app.clone(), *delay), *delay);
    }

    async fn handle_reconcile_requests(mut receiver: Receiver<K8sObject<ExposedApp>>) {
        let mut reconciler = Reconciler::new(K8sClient::new().await);
        let mut queue: DelayQueue<QueueEntry> = DelayQueue::with_capacity(32);
        let mut cached_versions: HashMap<NamespacedName, String> = HashMap::new();
        loop {
            cached_versions.clear();
            for _ in 0..10 {
                if let Ok(entry) = receiver.try_recv() {
                    let delay = Duration::from_secs(0);
                    queue.insert(QueueEntry::new(entry.clone(), delay), delay);
                    info!("Entry {} enqueued", entry.metadata.name.unwrap());
                } else {
                    break;
                }
            }
            while let Some(expired) = queue.next().await {
                let mut delay = expired.get_ref().delay.clone();
                let exposed_app = expired.get_ref().entry.clone();
                let name = exposed_app.metadata.name.clone().unwrap();
                let namespace = exposed_app.metadata.namespace.clone().unwrap();
                let namespaced_name = NamespacedName {
                    name,
                    namespace,
                };
                let resource_version = exposed_app.metadata.resource_version.clone().unwrap();
                info!("ExposedApp {} ready to reconcile", exposed_app.metadata.name.clone().unwrap());
                if let Some(version) = cached_versions.get_mut(&namespaced_name) {
                    if resource_version == *version {
                        info!("The same version received. Skip");
                    } else {
                        info!("Different version in cache. Reconcile");
                        match reconciler.reconcile(&exposed_app).await {
                            Ok(_) => {
                                *version = resource_version;
                            }
                            Err(err) => {
                                handle_reconcile_error(&mut delay, &mut queue, &exposed_app, err);
                            }
                        }
                    }
                } else {
                    info!("No version in cache. Reconcile");
                    match reconciler.reconcile(&exposed_app).await {
                        Ok(_) => {
                            cached_versions.insert(namespaced_name, resource_version);
                        }
                        Err(err) => {
                            handle_reconcile_error(&mut delay, &mut queue, &exposed_app, err);
                        }
                    }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}
