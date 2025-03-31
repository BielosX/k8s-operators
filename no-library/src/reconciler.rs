use crate::k8s_client::client::K8sClient;
use crate::k8s_types::{
    Container, ContainerPort, Deployment, DeploymentSpec, ExposedApp, K8sObject,
    Metadata, OwnerReference, PodSpec, PodTemplate, Selector, Service, ServicePort, ServiceSpec,
};
use std::collections::HashMap;
use tracing::info;

pub struct Reconciler {
    client: K8sClient,
}

impl Reconciler {
    pub fn new(client: K8sClient) -> Self {
        Reconciler { client }
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

    pub async fn reconcile(&mut self, resource: &K8sObject<ExposedApp>) -> Result<(), String> {
        let name = resource.metadata.name.clone().unwrap();
        let namespace = resource.metadata.namespace.clone().unwrap();
        info!("Synchronizing resource {} namespace {}", name, namespace,);
        let deployment_name = format!("{}-deployment", name);
        let pod_labels = HashMap::from([(
            String::from("app.kubernetes.io/instance"),
            deployment_name.clone(),
        )]);
        let deployment = K8sObject {
            api_version: String::from("apps/v1"),
            kind: String::from("Deployment"),
            metadata: Metadata {
                name: Some(deployment_name.clone()),
                namespace: Some(namespace.clone()),
                owner_references: Some(vec![Self::create_owner_reference(resource)]),
                ..Metadata::default()
            },
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
                            name: Some(name.clone()),
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
        match self.client.save_deployment(&deployment).await {
            Ok(_) => {
                info!("Deployment created/updated");
            }
            Err(e) => {
                return Err(format!("Error occurred while saving a deployment: {:?}", e));
            }
        }
        let service_name = format!("{}-service", name);
        let service = K8sObject {
            api_version: String::from("v1"),
            kind: String::from("Service"),
            metadata: Metadata {
                name: Some(service_name),
                namespace: Some(namespace.clone()),
                owner_references: Some(vec![Self::create_owner_reference(resource)]),
                ..Metadata::default()
            },
            object: Service {
                spec: ServiceSpec {
                    service_type: resource.object.spec.service_type.clone(),
                    selector: Some(pod_labels),
                    ports: vec![ServicePort {
                        protocol: resource.object.spec.protocol.clone(),
                        port: resource.object.spec.port,
                        target_port: resource.object.spec.container_port,
                        node_port: resource.object.spec.node_port,
                    }],
                },
            },
        };
        let put_service_result = self.client.put_service(&service).await;
        match put_service_result {
            Ok(_) => {
                info!("Service created/updated");
            }
            Err(e) => {
                return Err(format!("Error occurred while creating a service: {:?}", e));
            }
        }
        Ok(())
    }
}
