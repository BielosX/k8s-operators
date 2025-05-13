use crate::exposed_app::{ExposedApp, ExposedAppStatus};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int;
use kube::api::Patch::Merge;
use kube::api::{PatchParams, PostParams};
use kube::runtime::controller::Action;
use kube::{Api, Client, Error, Resource};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[derive(Clone)]
pub struct Data {
    client: Client,
}

impl Data {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

pub async fn reconcile(object: Arc<ExposedApp>, ctx: Arc<Data>) -> Result<Action, Error> {
    let namespace = object.metadata.namespace.clone().unwrap();
    let name = object.metadata.name.clone().unwrap();
    info!("Reconciling {} {}", name, namespace,);
    let services: Api<Service> = Api::namespaced(ctx.client.clone(), namespace.as_str());
    let deployments: Api<Deployment> = Api::namespaced(ctx.client.clone(), namespace.as_str());
    let exposed_apps: Api<ExposedApp> = Api::namespaced(ctx.client.clone(), namespace.as_str());
    let mut pod_labels: BTreeMap<String, String> = BTreeMap::new();
    pod_labels.insert(String::from("app.kubernetes.io/name"), name.clone());
    let deployment_name = format!("{}-deployment", name);
    let service_name = format!("{}-service", name);
    let new_deployment = deployment(
        deployment_name.as_str(),
        namespace.as_str(),
        &pod_labels,
        &object,
    );
    match deployments.get_opt(deployment_name.as_str()).await? {
        None => {
            deployments
                .create(&PostParams::default(), &new_deployment)
                .await?;
        }
        Some(_) => {
            deployments
                .patch(
                    deployment_name.as_str(),
                    &PatchParams::default(),
                    &Merge(&new_deployment),
                )
                .await?;
        }
    }
    /*
    patch_status(
        name.as_str(),
        &exposed_apps,
        ExposedAppStatus {
            deployment_name: Some(deployment_name.clone()),
            ..ExposedAppStatus::default()
        },
    )
    .await?;
     */
    let new_service = service(
        service_name.as_str(),
        namespace.as_str(),
        &pod_labels,
        &object,
    );
    match services.get_opt(service_name.as_str()).await? {
        None => {
            services
                .create(&PostParams::default(), &new_service)
                .await?;
        }
        Some(_) => {
            services
                .patch(
                    service_name.as_str(),
                    &PatchParams::default(),
                    &Merge(&new_service),
                )
                .await?;
        }
    }
    /*
    patch_status(
        name.as_str(),
        &exposed_apps,
        ExposedAppStatus {
            service_name: Some(service_name.clone()),
            ..ExposedAppStatus::default()
        },
    )
    .await?;
     */
    Ok(Action::await_change())
}

async fn patch_status(
    name: &str,
    api: &Api<ExposedApp>,
    app_status: ExposedAppStatus,
) -> Result<ExposedApp, Error> {
    let status = json!({
        "status": app_status,
    });
    api.patch_status(name, &PatchParams::default(), &Merge(&status))
        .await
}

fn deployment(
    name: &str,
    namespace: &str,
    pod_labels: &BTreeMap<String, String>,
    exposed_app: &ExposedApp,
) -> Deployment {
    let obj_ref = exposed_app.controller_owner_ref(&()).unwrap();
    Deployment {
        metadata: ObjectMeta {
            name: Some(String::from(name)),
            namespace: Some(String::from(namespace)),
            owner_references: Some(vec![obj_ref]),
            ..ObjectMeta::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(exposed_app.spec.replicas),
            selector: LabelSelector {
                match_labels: Some(pod_labels.clone()),
                ..LabelSelector::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(pod_labels.clone()),
                    ..ObjectMeta::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: String::from("main"),
                        image: Some(exposed_app.spec.image.clone()),
                        ports: Some(vec![ContainerPort {
                            protocol: Some(exposed_app.spec.protocol.clone()),
                            container_port: exposed_app.spec.container_port,
                            ..ContainerPort::default()
                        }]),
                        ..Container::default()
                    }],
                    ..PodSpec::default()
                }),
                ..PodTemplateSpec::default()
            },
            ..DeploymentSpec::default()
        }),
        ..Deployment::default()
    }
}

fn service(
    name: &str,
    namespace: &str,
    pod_labels: &BTreeMap<String, String>,
    exposed_app: &ExposedApp,
) -> Service {
    let obj_ref = exposed_app.controller_owner_ref(&()).unwrap();
    Service {
        metadata: ObjectMeta {
            name: Some(String::from(name)),
            namespace: Some(String::from(namespace)),
            owner_references: Some(vec![obj_ref]),
            ..ObjectMeta::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(pod_labels.clone()),
            type_: exposed_app.spec.service_type.clone(),
            ports: Some(vec![ServicePort {
                protocol: Some(exposed_app.spec.protocol.clone()),
                node_port: exposed_app.spec.node_port,
                target_port: Some(Int(exposed_app.spec.container_port)),
                port: exposed_app.spec.port,
                ..ServicePort::default()
            }]),
            ..ServiceSpec::default()
        }),
        ..Service::default()
    }
}

pub fn error_policy(_object: Arc<ExposedApp>, _err: &Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(Duration::from_secs(5))
}
