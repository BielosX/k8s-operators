use crate::k8s_client::client::{K8sClient, K8sClientError};
use std::num::NonZeroU8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use time::format_description::well_known::iso8601::{Config, EncodedConfig, TimePrecision};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::{error, info};

const LEASE_NAME: &str = "no-library";
const LEASE_NAMESPACE: &str = "no-library";

pub struct LeaderElector {
    client: K8sClient,
    pod_id: String,
    is_leader_elected: Arc<AtomicBool>,
    is_leader_sender: Sender<()>,
}

impl LeaderElector {
    pub fn new(
        client: K8sClient,
        pod_id: &str,
        is_leader_elected: Arc<AtomicBool>,
        is_leader_sender: Sender<()>,
    ) -> Self {
        LeaderElector {
            client,
            pod_id: String::from(pod_id),
            is_leader_elected,
            is_leader_sender,
        }
    }

    pub async fn elect_leader(&mut self) {
        let mut is_leader = false;
        loop {
            let lease = self
                .client
                .get_lease(LEASE_NAMESPACE, LEASE_NAME)
                .await
                .expect("Lease should be there, better check yaml config");
            let duration = lease.object.spec.lease_duration_seconds;
            let seconds = duration >> 1;
            let now = OffsetDateTime::now_utc();
            const CONFIG: EncodedConfig = Config::DEFAULT
                .set_time_precision(TimePrecision::Second {
                    decimal_digits: NonZeroU8::new(6),
                })
                .encode();
            let resource_version = lease.metadata.resource_version.clone().unwrap();
            let can_acquire = lease
                .object
                .spec
                .acquire_time
                .clone()
                .map(|t| {
                    let date = OffsetDateTime::parse(t.as_str(), &Iso8601::<CONFIG>).unwrap();
                    now > date
                        .checked_add(time::Duration::seconds(duration as i64))
                        .unwrap()
                })
                .unwrap_or(true);
            if is_leader {
                info!("Refreshing lease");
                self.client
                    .patch_lease(
                        LEASE_NAMESPACE,
                        LEASE_NAME,
                        resource_version.as_str(),
                        self.pod_id.as_str(),
                        now,
                    )
                    .await
                    .unwrap();
            } else if can_acquire {
                info!("Trying to acquire the lease");
                let acquire_result = self
                    .client
                    .patch_lease(
                        LEASE_NAMESPACE,
                        LEASE_NAME,
                        resource_version.as_str(),
                        self.pod_id.as_str(),
                        now,
                    )
                    .await;
                match acquire_result {
                    Ok(_) => {
                        info!("Became a leader");
                        if !is_leader {
                            self.is_leader_sender.send(()).await.unwrap();
                            self.is_leader_elected.store(true, Ordering::SeqCst);
                        }
                        is_leader = true;
                    }
                    Err(K8sClientError::Conflict) => {
                        info!("Some other instance updated the lease");
                        self.is_leader_elected.store(true, Ordering::SeqCst);
                    }
                    Err(_) => {
                        error!("Something went wrong during leader election");
                    }
                }
            }
            info!("Waiting for {}", seconds);
            sleep(std::time::Duration::from_secs(seconds as u64)).await;
        }
    }
}
