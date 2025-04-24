use crate::cache::{Cache, CacheEntry, NamespacedName};
use crate::k8s_client::client::{K8sClient, K8sClientError};
use crate::k8s_types::EventType::Normal;
use crate::k8s_types::{
    Container, ContainerPort, Deployment, DeploymentSpec, Event, EventType, ExposedApp,
    ExposedAppStatus, K8sObject, Metadata, ObjectReference, OwnerReference, PodSpec, PodTemplate,
    Selector, Service, ServicePort, ServiceSpec,
};
use crate::offset_date_time_parser::format;
use rand::distr::{Alphanumeric, SampleString};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::info;

pub struct Reconciler {
    client: K8sClient,
    cache: Cache,
    pod_name: String,
}

type PodLabels = HashMap<String, String>;

impl Reconciler {
    pub fn new(client: K8sClient, cache: Cache, pod_name: String) -> Self {
        Reconciler {
            client,
            cache,
            pod_name,
        }
    }

    fn create_owner_reference(resource: &K8sObject<ExposedApp>) -> OwnerReference {
        OwnerReference {
            api_version: resource.api_version.clone(),
            kind: resource.kind.clone(),
            name: resource.metadata.name.clone().unwrap(),
            uid: resource.metadata.uid.clone().unwrap(),
            block_owner_deletion: true,
            controller: true,
        }
    }

    fn metadata(name: &str, namespace: &str, resource: &K8sObject<ExposedApp>) -> Metadata {
        Metadata {
            name: Some(String::from(name)),
            namespace: Some(String::from(namespace)),
            owner_references: Some(vec![Self::create_owner_reference(resource)]),
            ..Metadata::default()
        }
    }

    fn random_str(len: usize) -> String {
        Alphanumeric.sample_string(&mut rand::rng(), len)
    }

    async fn send_event(
        &mut self,
        resource: &K8sObject<ExposedApp>,
        related: &ObjectReference,
        event_type: EventType,
        action: &str,
        note: &str,
        reason: &str,
    ) -> Result<K8sObject<Event>, K8sClientError> {
        let resource_name = resource.metadata.name.clone().unwrap();
        let namespace = resource.metadata.namespace.clone().unwrap();
        let suffix = Self::random_str(10).to_lowercase();
        let name = format!("{}-{}", resource_name, suffix);
        let event_time = format(OffsetDateTime::now_utc()).unwrap();
        let event = K8sObject {
            api_version: String::from("events.k8s.io/v1"),
            kind: String::from("Event"),
            metadata: Metadata {
                name: Some(name),
                namespace: Some(namespace.clone()),
                ..Metadata::default()
            },
            object: Event {
                event_time,
                action: String::from(action),
                note: Some(String::from(note)),
                reason: String::from(reason),
                regarding: Some(resource.into()),
                related: Some(related.clone()),
                reporting_controller: String::from("no-library"),
                reporting_instance: self.pod_name.clone(),
                event_type,
            },
        };
        self.client.post_event(namespace.as_str(), &event).await
    }

    async fn save_deployment(
        &mut self,
        name: &str,
        namespace: &str,
        pod_labels: &PodLabels,
        resource: &K8sObject<ExposedApp>,
    ) -> Result<K8sObject<Deployment>, String> {
        let deployment = K8sObject {
            api_version: String::from("apps/v1"),
            kind: String::from("Deployment"),
            metadata: Self::metadata(name, namespace, resource),
            object: Deployment {
                spec: DeploymentSpec {
                    replicas: resource.object.spec.replicas,
                    selector: {
                        Selector {
                            match_labels: pod_labels.clone(),
                        }
                    },
                    template: PodTemplate {
                        metadata: Metadata {
                            labels: Some(pod_labels.clone()),
                            name: Some(String::from(name)),
                            ..Metadata::default()
                        },
                        spec: PodSpec {
                            containers: vec![Container {
                                name: String::from("main"),
                                image: resource.object.spec.image.clone(),
                                ports: Some(vec![ContainerPort {
                                    container_port: resource.object.spec.container_port,
                                }]),
                            }],
                        },
                    },
                },
            },
        };
        let mut map = self.cache.lock().await;
        match self.client.save_deployment(&deployment).await {
            Ok(result) => {
                info!("Deployment created/updated");
                map.insert(
                    NamespacedName::new(name, namespace),
                    CacheEntry::new(
                        result.metadata.resource_version.clone().unwrap().as_str(),
                        result.metadata.generation.unwrap(),
                    ),
                );
                Ok(result)
            }
            Err(e) => Err(format!("Error occurred while saving a deployment: {:?}", e)),
        }
    }

    async fn save_service(
        &mut self,
        name: &str,
        namespace: &str,
        pod_labels: &PodLabels,
        resource: &K8sObject<ExposedApp>,
    ) -> Result<K8sObject<Service>, String> {
        let service = K8sObject {
            api_version: String::from("v1"),
            kind: String::from("Service"),
            metadata: Self::metadata(name, namespace, resource),
            object: Service {
                spec: ServiceSpec {
                    service_type: resource.object.spec.service_type.clone(),
                    selector: Some(pod_labels.clone()),
                    ports: vec![ServicePort {
                        protocol: resource.object.spec.protocol.clone(),
                        port: resource.object.spec.port,
                        target_port: resource.object.spec.container_port,
                        node_port: resource.object.spec.node_port,
                    }],
                },
            },
        };
        let mut map = self.cache.lock().await;
        /*
           Service does not provide generation,
           there’s no controller watching the spec and updating status in a way that would need generation tracking.
        */
        match self.client.put_service(&service).await {
            Ok(result) => {
                info!("Service created/updated");
                map.insert(
                    NamespacedName::new(name, namespace),
                    CacheEntry::new_no_generation(
                        result.metadata.resource_version.clone().unwrap().as_str(),
                    ),
                );
                Ok(result)
            }
            Err(e) => Err(format!("Error occurred while creating a service: {:?}", e)),
        }
    }

    async fn reconcile_resource(
        &mut self,
        resource: &mut K8sObject<ExposedApp>,
    ) -> Result<(), String> {
        let name = resource.metadata.name.clone().unwrap();
        let namespace = resource.metadata.namespace.clone().unwrap();
        info!("Synchronizing resource {} namespace {}", name, namespace);
        let deployment_name = format!("{}-deployment", name);
        let pod_labels = HashMap::from([(
            String::from("app.kubernetes.io/instance"),
            deployment_name.clone(),
        )]);
        match self
            .save_deployment(
                deployment_name.as_str(),
                namespace.as_str(),
                &pod_labels,
                resource,
            )
            .await
        {
            Ok(deployment) => {
                let note = format!(
                    "Deployment {} provisioned successfully with {} replicas",
                    deployment.metadata.name.clone().unwrap(),
                    deployment.object.spec.replicas
                );
                self.send_event(
                    resource,
                    &deployment.into(),
                    Normal,
                    "DeploymentProvisioned",
                    note.as_str(),
                    "ProvisioningRequested",
                )
                .await
                .unwrap();
            }
            Err(e) => return Err(e),
        }
        let service_name = format!("{}-service", name);
        match self
            .save_service(
                service_name.as_str(),
                namespace.as_str(),
                &pod_labels,
                resource,
            )
            .await
        {
            Ok(service) => {
                let note = format!(
                    "Service {} successfully provisioned",
                    service.metadata.name.clone().unwrap()
                );
                self.send_event(
                    resource,
                    &service.into(),
                    Normal,
                    "ServiceProvisioned",
                    note.as_str(),
                    "ProvisioningRequested",
                )
                .await
                .unwrap();
            }
            Err(e) => return Err(e),
        }
        resource.object.status = Some(ExposedAppStatus {
            deployment_name,
            service_name,
        });
        let mut map = self.cache.lock().await;
        match self
            .client
            .put_exposed_app_status(namespace.as_str(), name.as_str(), &resource)
            .await
        {
            Ok(result) => {
                info!("Successfully updated status of {}", name);
                let namespaced_name = NamespacedName::new(name.as_str(), namespace.as_str());
                map.insert(
                    namespaced_name,
                    CacheEntry::new(
                        result.metadata.resource_version.clone().unwrap().as_str(),
                        result.metadata.generation.unwrap(),
                    ),
                );
            }
            Err(e) => {
                return Err(format!(
                    "Error occurred while updating ExposedApp status: {:?}",
                    e
                ));
            }
        }
        Ok(())
    }

    pub async fn reconcile(&mut self, namespaced_name: NamespacedName) -> Result<(), String> {
        let name = namespaced_name.name;
        let namespace = namespaced_name.namespace;
        match self
            .client
            .get_exposed_app(name.as_str(), namespace.as_str())
            .await
        {
            Ok(mut resource) => self.reconcile_resource(&mut resource).await,
            Err(K8sClientError::NotFound) => {
                info!("ExposedApp not found, probably already deleted. It's fine");
                Ok(())
            }
            Err(e) => Err(format!("Unable to get ExposedApp: {:?}", e)),
        }
    }
}
