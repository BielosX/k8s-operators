use crate::k8s_client::client::{K8sClient, K8sClientError};
use crate::k8s_types::{
    Container, ContainerPort, Deployment, DeploymentSpec, ExposedApp, K8sObject, Metadata, PodSpec,
    PodTemplate, Selector, Service, ServicePort, ServiceSpec,
};
use std::collections::HashMap;
use tracing::{error, info};

const FINALIZER_NAME: &str = "exposedapps.stable.no-library.com/finalizer";

pub struct Reconciler {
    client: K8sClient,
}

impl Reconciler {
    pub fn new(client: K8sClient) -> Self {
        Reconciler { client }
    }

    async fn clean_up(&mut self, resource: &K8sObject<ExposedApp>) {
        let name = resource.metadata.name.clone().unwrap();
        let namespace = resource.metadata.namespace.clone().unwrap();
        info!(
            "Deleting related resources for {} namespace {}",
            name, namespace
        );
        let deployment_name = format!("{}-deployment", name);
        let deployment_delete_result = self
            .client
            .delete(
                namespace.as_str(),
                "deployments",
                "apps",
                "v1",
                deployment_name.as_str(),
            )
            .await;
        match deployment_delete_result {
            Ok(_) => {}
            Err(K8sClientError::NotFound) => {
                info!("Deployment {} not found, that's fine", deployment_name);
            }
            Err(_) => {
                error!("Something wrong happened while trying to delete deployment");
            }
        }
        let service_name = format!("{}-service", name);
        let service_delete_result = self
            .client
            .delete_uri(
                format!("/api/v1/namespaces/{}/services/{}", namespace, service_name).as_str(),
            )
            .await;
        match service_delete_result {
            Ok(_) => {}
            Err(K8sClientError::NotFound) => {
                info!("Service {} not found, that's fine", deployment_name);
            }
            Err(_) => {
                error!("Something wrong happened while trying to delete service");
            }
        }
    }

    async fn handle_finalizer(&mut self, resource: &K8sObject<ExposedApp>) -> bool {
        let mut resource_copy: K8sObject<ExposedApp> = resource.clone();
        match &resource.metadata.finalizers {
            None => {
                info!("Adding finalizer");
                resource_copy.metadata.finalizers = Some(vec![FINALIZER_NAME.to_string()]);
                self.client.put_exposed_app(&resource_copy).await.unwrap();
                false
            }
            Some(finalizers) => {
                if resource.metadata.deletion_timestamp.is_some() {
                    info!("Removing finalizer");
                    if finalizers.len() == 1 {
                        info!("{} is the only one finalizer", FINALIZER_NAME);
                        resource_copy.metadata.finalizers = None;
                    } else {
                        info!(
                            "More than one finalizer found, removing {} only",
                            FINALIZER_NAME
                        );
                        let new_finalizers: Vec<String> = finalizers
                            .iter()
                            .filter(|&i| i != FINALIZER_NAME)
                            .map(Clone::clone)
                            .collect();
                        resource_copy.metadata.finalizers = Some(new_finalizers);
                    }
                    self.clean_up(&resource_copy).await;
                    match self.client.put_exposed_app(&resource_copy).await {
                        Ok(_) => {}
                        Err(K8sClientError::NotFound) => {
                            info!("Nothing to update, it's fine");
                        }
                        Err(_) => {
                            error!("Something wrong happened");
                        }
                    }
                    true
                } else {
                    false
                }
            }
        }
    }

    pub async fn reconcile(&mut self, resource: &K8sObject<ExposedApp>) {
        let finalized = self.handle_finalizer(resource).await;
        if !finalized {
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
                                    ports: vec![ContainerPort {
                                        container_port: resource.object.spec.container_port,
                                    }],
                                }],
                            },
                        },
                    },
                },
            };
            let post_deployment_result = self.client.post_deployment(&deployment).await;
            match post_deployment_result {
                Ok(_) => {}
                Err(K8sClientError::Conflict) => {
                    info!(
                        "Deployment {} already exists, updating",
                        deployment_name.clone()
                    );
                    self.client.put_deployment(&deployment).await.unwrap();
                }
                Err(e) => {
                    error!("Error occurred while creating a deployment: {:?}", e);
                }
            }
            let service_name = format!("{}-service", name);
            let service = K8sObject {
                api_version: String::from("v1"),
                kind: String::from("Service"),
                metadata: Metadata {
                    name: Some(service_name),
                    namespace: Some(namespace),
                    ..Metadata::default()
                },
                object: Service {
                    spec: ServiceSpec {
                        service_type: resource.object.spec.service_type.clone(),
                        selector: pod_labels,
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
                    error!("Error occurred while creating a service: {:?}", e);
                }
            }
        }
    }
}
