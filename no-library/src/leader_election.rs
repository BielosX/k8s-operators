use crate::k8s_client::client::{K8sClient, K8sClientError};
use crate::k8s_types::{K8sObject, Lease};
use crate::offset_date_time_parser::parse;
use time::{Duration, OffsetDateTime};
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::{error, info};

const LEASE_NAME: &str = "no-library";
const LEASE_NAMESPACE: &str = "no-library";

pub struct LeaderElector {
    client: K8sClient,
    pod_id: String,
    is_leader_sender: Sender<()>,
}

impl LeaderElector {
    pub fn new(client: K8sClient, pod_id: &str, is_leader_sender: Sender<()>) -> Self {
        LeaderElector {
            client,
            pod_id: String::from(pod_id),
            is_leader_sender,
        }
    }

    async fn get_lease(&mut self) -> K8sObject<Lease> {
        self.client
            .get_lease(LEASE_NAME, LEASE_NAMESPACE)
            .await
            .expect("Failed to get lease, should be available. Check yaml config")
    }

    async fn patch_lease(
        &mut self,
        version: &str,
        now: &OffsetDateTime,
    ) -> Result<(), K8sClientError> {
        self.client
            .patch_lease(
                LEASE_NAMESPACE,
                LEASE_NAME,
                version,
                self.pod_id.as_str(),
                now.clone(),
            )
            .await
    }

    async fn acquire_lease(&mut self) {
        loop {
            let lease = self.get_lease().await;
            let duration = lease.object.spec.lease_duration_seconds;
            let now = OffsetDateTime::now_utc();
            let resource_version = lease.metadata.resource_version.clone().unwrap();
            let acquired_by = lease.object.spec.holder_identity.clone();
            let can_acquire = lease
                .object
                .spec
                .acquire_time
                .map(|t| {
                    now > parse(t.as_str())
                        .unwrap()
                        .checked_add(Duration::seconds(duration as i64))
                        .unwrap()
                })
                .unwrap_or(true);
            if can_acquire {
                info!("Trying to acquire lease {}", LEASE_NAME);
                match self.patch_lease(resource_version.as_str(), &now).await {
                    Ok(_) => {
                        info!("Lease acquired, became a leader");
                        self.is_leader_sender.send(()).await.unwrap();
                        return;
                    }
                    Err(e) => {
                        info!("Failed to acquire lease. Reason: {:?}", e);
                    }
                }
            } else {
                info!(
                    "Lease already acquired by {}, waiting",
                    acquired_by.unwrap()
                );
            }
            sleep(std::time::Duration::from_secs(duration as u64)).await;
        }
    }

    async fn refresh_lease(&mut self) {
        loop {
            let lease = self.get_lease().await;
            let duration = lease.object.spec.lease_duration_seconds;
            let wait_time = duration >> 1;
            let resource_version = lease.metadata.resource_version.clone().unwrap();
            info!("Refreshing lease {}", LEASE_NAME);
            let now = OffsetDateTime::now_utc();
            match self.patch_lease(resource_version.as_str(), &now).await {
                Ok(_) => {
                    info!("Lease refreshed");
                }
                Err(e) => {
                    error!("Failed to refresh lease. Reason: {:?}", e);
                }
            }
            sleep(std::time::Duration::from_secs(wait_time as u64)).await;
        }
    }

    pub async fn elect_leader(&mut self) {
        self.acquire_lease().await;
        self.refresh_lease().await;
    }
}
