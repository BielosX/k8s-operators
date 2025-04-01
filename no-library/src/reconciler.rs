use crate::cache::{Cache, NamespacedName};
use crate::k8s_client::client::K8sClient;
use crate::k8s_types::{
    Container, ContainerPort, Deployment, DeploymentSpec, ExposedApp, ExposedAppStatus, K8sObject,
    Metadata, OwnerReference, PodSpec, PodTemplate, Selector, Service, ServicePort, ServiceSpec,
};
use std::collections::HashMap;
use tracing::info;

pub struct Reconciler {
    client: K8sClient,
    cache: Cache,
}

type PodLabels = HashMap<String, String>;

impl Reconciler {
    pub fn new(client: K8sClient, cache: Cache) -> Self {
        Reconciler { client, cache }
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

    async fn save_deployment(
        &mut self,
        name: &str,
        namespace: &str,
        pod_labels: &PodLabels,
        resource: &K8sObject<ExposedApp>,
    ) -> Result<(), String> {
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
                    result.metadata.resource_version.unwrap(),
                );
                Ok(())
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
    ) -> Result<(), String> {
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
        match self.client.put_service(&service).await {
            Ok(result) => {
                info!("Service created/updated");
                map.insert(
                    NamespacedName::new(name, namespace),
                    result.metadata.resource_version.unwrap(),
                );
                Ok(())
            }
            Err(e) => Err(format!("Error occurred while creating a service: {:?}", e)),
        }
    }

    pub async fn reconcile(&mut self, resource: &mut K8sObject<ExposedApp>) -> Result<(), String> {
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
            Ok(_) => {}
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
            Ok(_) => {}
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
                    result.metadata.resource_version.clone().unwrap(),
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
}
