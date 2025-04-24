pub mod operator {
    use crate::cache::{clone_cache, new_cache, Cache, NamespacedName};
    use crate::k8s_client::client::{K8sClient, K8sClientError};
    use crate::k8s_types::{
        Deployment, ExposedApp, K8sListObject, K8sObject, MetadataAware, Service,
    };
    use crate::leader_election::LeaderElector;
    use crate::reconciler::Reconciler;
    use futures::{pin_mut, StreamExt};
    use serde::de::DeserializeOwned;
    use std::ops::Mul;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::select;
    use tokio::sync::mpsc::{Receiver, Sender};
    use tokio::sync::{mpsc, Notify};
    use tokio::time::sleep;
    use tokio_util::time::DelayQueue;
    use tracing::{error, info, warn};

    const ALL_DEPLOYMENTS_LIST: &str = "apis/apps/v1/deployments";
    const ALL_SERVICES_LIST: &str = "api/v1/services";

    pub async fn elect_leader(pod_id: String, is_leader_sender: Arc<Notify>) {
        let mut leader_elector =
            LeaderElector::new(K8sClient::new().await, pod_id.as_str(), is_leader_sender);
        leader_elector.elect_leader().await;
    }

    pub async fn handle_owned_resources(notify: Arc<Notify>, pod_name: String) {
        notify.notified().await;
        info!("Started doing operator stuff");
        let (app_sender, app_receiver): (
            Sender<K8sObject<ExposedApp>>,
            Receiver<K8sObject<ExposedApp>>,
        ) = mpsc::channel(64);
        let cache = new_cache();
        select! {
            _ = tokio::spawn(handle_reconcile_requests(app_receiver, clone_cache(&cache), pod_name)) => {}
            _ = tokio::spawn(handle_exposed_apps(app_sender.clone(), clone_cache(&cache))) => {}
            _ = tokio::spawn(handle_owned_update::<Deployment>(app_sender.clone(), ALL_DEPLOYMENTS_LIST, clone_cache(&cache))) => {}
            _ = tokio::spawn(handle_owned_update::<Service>(app_sender.clone(), ALL_SERVICES_LIST, clone_cache(&cache))) => {}
        }
    }

    struct QueueEntry {
        entry: NamespacedName,
        delay: Duration,
    }

    impl QueueEntry {
        pub fn new(entry: NamespacedName, delay: Duration) -> Self {
            QueueEntry { entry, delay }
        }
    }

    async fn can_skip_reconcile<T>(object: &impl MetadataAware, cache: &Cache) -> bool {
        let name = object.metadata().name.unwrap();
        let namespace = object.metadata().namespace.unwrap();
        let namespaced_name = NamespacedName::new(name.as_str(), namespace.as_str());
        let resource_version = object.metadata().resource_version.unwrap();
        let generation = object.metadata().generation;
        let map = cache.lock().await;
        if let Some(value) = map.get(&namespaced_name) {
            match (generation, value.generation) {
                (Some(left), Some(right)) => {
                    if left == right {
                        info!("Object {} generation {}. Status updated, skip.", name, left);
                        return true;
                    }
                }
                (_, _) => {}
            }
            if resource_version == value.resource_version {
                info!(
                    "Object {} version {} already handled, skip",
                    name, resource_version
                );
                return true;
            }
        }
        false
    }

    async fn handle_exposed_apps(sender: Sender<K8sObject<ExposedApp>>, cache: Cache) {
        let mut client = K8sClient::new().await;
        loop {
            info!("Watching ExposedApp");
            match client.get_exposed_apps().await {
                Ok(get_result) => {
                    let resource_version = get_result.metadata.resource_version.clone().unwrap();
                    for item in get_result.items {
                        sender.send(item).await.unwrap();
                    }
                    match client.watch_exposed_apps(resource_version.as_str()).await {
                        Ok(stream) => {
                            pin_mut!(stream);
                            while let Some(event) = stream.next().await {
                                info!("Received ExposedApp {:?} event", event.event_type);
                                let name = event.object.metadata.name.clone().unwrap();
                                if can_skip_reconcile::<K8sListObject<ExposedApp>>(
                                    &event.object,
                                    &cache,
                                )
                                .await
                                {
                                    continue;
                                }
                                info!("Sending reconcile event for {}", name);
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
                            warn!("ExposedApp stream closed. Will retry");
                        }
                        Err(e) => {
                            error!("Error occurred while trying to watch ExposedApps: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Error occurred while trying to get ExposedApps: {:?}", e);
                }
            }
            sleep(Duration::from_secs(2)).await;
        }
    }

    async fn handle_owned_update<T: DeserializeOwned>(
        sender: Sender<K8sObject<ExposedApp>>,
        uri: &str,
        cache: Cache,
    ) {
        let mut client = K8sClient::new().await;
        loop {
            info!("Watching URI {}", uri);
            match client.get_all::<T>(uri).await {
                Ok(result) => {
                    let resource_version = result.metadata.resource_version.clone().unwrap();
                    match client.watch::<T>(uri, resource_version.as_str()).await {
                        Ok(stream) => {
                            pin_mut!(stream);
                            while let Some(event) = stream.next().await {
                                let object_name = event.object.metadata.name.clone().unwrap();
                                let version =
                                    event.object.metadata.resource_version.clone().unwrap();
                                info!(
                                    "Received {:?} event for {}, version {}",
                                    event.event_type, object_name, version
                                );
                                let namespace = event.object.metadata.namespace.clone().unwrap();
                                if can_skip_reconcile::<K8sObject<T>>(&event.object, &cache).await {
                                    continue;
                                }
                                info!("Looking for owner references for {}", object_name);
                                if let Some(references) = event.object.metadata.owner_references {
                                    if let Some(exposed_app_ref) =
                                        references.iter().find(|&item| item.kind == "ExposedApp")
                                    {
                                        let owner_name = exposed_app_ref.name.clone();
                                        info!(
                                            "Found ExposedApp owner {} for object {}",
                                            owner_name, object_name
                                        );
                                        match client
                                            .get_exposed_app(
                                                owner_name.as_str(),
                                                namespace.as_str(),
                                            )
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
                            warn!("{} stream closed. Will retry", uri);
                        }
                        Err(e) => {
                            error!(
                                "Error occurred while trying to watch Owned Resource: {:?}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Error occurred while trying to get Owned Resource: {:?}", e);
                }
            }
            sleep(Duration::from_secs(2)).await;
        }
    }

    async fn handle_reconcile_requests(
        mut receiver: Receiver<K8sObject<ExposedApp>>,
        cache: Cache,
        pod_name: String,
    ) {
        let mut reconciler = Reconciler::new(K8sClient::new().await, cache, pod_name);
        let mut queue: DelayQueue<QueueEntry> = DelayQueue::with_capacity(32);
        loop {
            for _ in 0..10 {
                if let Ok(entry) = receiver.try_recv() {
                    let delay = Duration::from_secs(0);
                    let name = entry.metadata.name.clone().unwrap();
                    let namespace = entry.metadata.namespace.clone().unwrap();
                    queue.insert(
                        QueueEntry::new(
                            NamespacedName::new(name.as_str(), namespace.as_str()),
                            delay,
                        ),
                        delay,
                    );
                    info!("Entry {} enqueued", entry.metadata.name.unwrap());
                } else {
                    break;
                }
            }
            while let Some(expired) = queue.next().await {
                let mut delay = expired.get_ref().delay.clone();
                let namespaced_name = NamespacedName::new(
                    expired.get_ref().entry.name.as_str(),
                    expired.get_ref().entry.namespace.as_str(),
                );
                info!("ExposedApp {} ready to reconcile", namespaced_name.name);
                match reconciler.reconcile(namespaced_name.clone()).await {
                    Ok(_) => {
                        info!(
                            "ExposedApp {} successfully reconciled",
                            namespaced_name.name
                        );
                    }
                    Err(err) => {
                        error!(err);
                        if delay.is_zero() {
                            delay = Duration::from_secs(2);
                        } else {
                            if delay < Duration::from_secs(128) {
                                delay = delay.mul(2);
                            }
                        }
                        queue.insert(QueueEntry::new(namespaced_name, delay), delay);
                    }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}
