pub mod client {
    use crate::k8s_client::client::K8sClientError::{Conflict, Error, NotFound};
    use crate::k8s_types::{
        Deployment, Event, ExposedApp, K8sListObject, K8sObject, Lease, List, Service, Watch,
    };
    use crate::offset_date_time_parser::format;
    use async_stream::stream;
    use futures::Stream;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::{Certificate, Client, RequestBuilder, Response, StatusCode};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use serde_json::{from_str, to_string};
    use std::env;
    use std::str::from_utf8;
    use std::time::Duration;
    use time::OffsetDateTime;
    use tokio::fs;
    use tokio::time::sleep;
    use tracing::{error, info};

    const SERVICE_ACCOUNT_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount";
    const API_SERVER: &str = "https://kubernetes.default.svc";
    const EXPOSED_APPS_LIST: &str = "apis/stable.no-library.com/v1/exposedapps";

    async fn get_token() -> String {
        let content = fs::read(format!("{}/token", SERVICE_ACCOUNT_PATH))
            .await
            .expect("Unable to read token");
        String::from_utf8(content).expect("Unable to convert token to UTF8")
    }

    async fn get_client() -> Client {
        let cert = fs::read(format!("{}/ca.crt", SERVICE_ACCOUNT_PATH))
            .await
            .expect("Unable to read certificate");
        let certificate =
            Certificate::from_pem(cert.as_slice()).expect("Unable to create Certificate");
        Client::builder()
            .add_root_certificate(certificate)
            .build()
            .expect("Unable to add root certificate")
    }

    #[derive(Debug, Clone)]
    pub enum K8sClientError {
        NotFound,
        Conflict,
        #[allow(dead_code)]
        Error(String),
    }

    impl K8sClientError {
        pub fn from_status(status: StatusCode, text: &str) -> Option<K8sClientError> {
            match status.as_u16() {
                404 => Some(NotFound),
                409 => Some(Conflict),
                _ => {
                    if !status.is_success() {
                        Some(Error(String::from(text)))
                    } else {
                        None
                    }
                }
            }
        }
    }

    #[derive(Clone)]
    pub struct K8sClient {
        client: Client,
        token: String,
        k8s_service_host: String,
        k8s_service_port: u16,
    }

    #[derive(Serialize, Deserialize)]
    struct JsonPatchEntry<T> {
        op: String,
        path: String,
        value: T,
    }

    impl K8sClient {
        pub async fn new() -> Self {
            let host = env::var("KUBERNETES_SERVICE_HOST").unwrap_or(String::new());
            let port = env::var("KUBERNETES_SERVICE_PORT")
                .map(|p| p.parse::<u16>().unwrap())
                .unwrap_or(0);
            if !host.is_empty() {
                info!(
                    "Using KUBERNETES_SERVICE_HOST: {} and KUBERNETES_SERVICE_PORT: {}",
                    host, port
                );
            }
            K8sClient {
                client: get_client().await,
                token: get_token().await,
                k8s_service_host: host,
                k8s_service_port: port,
            }
        }

        fn get_api_server_url(&self) -> String {
            if self.k8s_service_host.is_empty() {
                String::from(API_SERVER)
            } else {
                format!(
                    "https://{}:{}",
                    self.k8s_service_host, self.k8s_service_port
                )
            }
        }

        fn get_headers(&self) -> HeaderMap {
            let mut headers = HeaderMap::new();
            let value = HeaderValue::from_str(format!("Bearer {}", self.token).as_str()).unwrap();
            headers.insert("Authorization", value);
            headers.insert(
                "User-Agent",
                HeaderValue::from_str("exposed-apps-controller").unwrap(),
            );
            headers
        }

        pub async fn get_exposed_apps(
            &mut self,
        ) -> Result<List<K8sObject<ExposedApp>>, K8sClientError> {
            let result = self
                .send_with_retry(self.client.get(format!(
                    "{}/{}",
                    self.get_api_server_url(),
                    EXPOSED_APPS_LIST
                )))
                .await;
            let status = result.status();
            let text = result.text().await.unwrap();
            K8sClientError::from_status(status, text.as_str())
                .map(Err)
                .unwrap_or_else(|| {
                    let object = from_str::<List<K8sObject<ExposedApp>>>(text.as_str()).unwrap();
                    Ok(object)
                })
        }

        pub async fn get_exposed_app(
            &mut self,
            name: &str,
            namespace: &str,
        ) -> Result<K8sObject<ExposedApp>, K8sClientError> {
            let response = self
                .send_with_retry(self.client.get(format!(
                    "{}/apis/stable.no-library.com/v1/namespaces/{}/exposedapps/{}",
                    self.get_api_server_url(),
                    namespace,
                    name
                )))
                .await;
            let status = response.status();
            let text = response.text().await.unwrap();
            K8sClientError::from_status(status, text.as_str())
                .map(Err)
                .unwrap_or_else(|| {
                    let object = from_str::<K8sObject<ExposedApp>>(text.as_str()).unwrap();
                    Ok(object)
                })
        }

        /*
          https://kubernetes.io/docs/reference/using-api/api-concepts/#api-verbs
          For PUT requests, Kubernetes internally classifies these as
          either create or update based on the state of the existing object.

          IT'S NOT TRUE FOR DEPLOYMENT
          WORKS FINE FOR SERVICE
        */
        pub async fn put<T: Serialize + DeserializeOwned>(
            &mut self,
            item: &K8sObject<T>,
            namespace: &str,
            resource_type: &str,
            group: &str,
            version: &str,
            name: &str,
        ) -> Result<K8sObject<T>, K8sClientError> {
            let url = format!(
                "{}/apis/{}/{}/namespaces/{}/{}/{}",
                self.get_api_server_url(),
                group,
                version,
                namespace,
                resource_type,
                name
            );
            self.put_with_url(item, url.as_str()).await
        }

        async fn put_with_url<T: Serialize + DeserializeOwned>(
            &mut self,
            item: &K8sObject<T>,
            url: &str,
        ) -> Result<K8sObject<T>, K8sClientError> {
            self.execute(self.client.put(url), item).await
        }

        pub async fn post<T: Serialize + DeserializeOwned>(
            &mut self,
            item: &K8sObject<T>,
            namespace: &str,
            resource_type: &str,
            group: &str,
            version: &str,
        ) -> Result<K8sObject<T>, K8sClientError> {
            let url = format!(
                "{}/apis/{}/{}/namespaces/{}/{}",
                self.get_api_server_url(),
                group,
                version,
                namespace,
                resource_type,
            );
            self.execute(self.client.post(url), item).await
        }

        async fn refresh_token(&mut self) {
            self.token = get_token().await;
        }

        async fn send_with_retry(&mut self, builder: RequestBuilder) -> Response {
            let mut result = builder
                .try_clone()
                .unwrap()
                .headers(self.get_headers())
                .send()
                .await
                .unwrap();
            let mut last_status = result.status().as_u16();
            let mut retries = 3;
            while last_status == 401 && retries > 0 {
                info!("Refreshing token");
                sleep(Duration::from_secs(10)).await;
                self.refresh_token().await;
                let response = builder
                    .try_clone()
                    .unwrap()
                    .headers(self.get_headers())
                    .send()
                    .await
                    .unwrap();
                last_status = response.status().as_u16();
                result = response;
                retries = retries - 1;
            }
            result
        }

        async fn execute<I: Serialize, O: DeserializeOwned>(
            &mut self,
            builder: RequestBuilder,
            item: &I,
        ) -> Result<O, K8sClientError> {
            let payload = to_string(&item).unwrap();
            let result = self.send_with_retry(builder.body(payload)).await;
            let status = result.status();
            let text = result.text().await.unwrap();
            K8sClientError::from_status(status, text.as_str())
                .map(Err)
                .unwrap_or_else(|| {
                    let object = from_str::<O>(text.as_str()).unwrap();
                    Ok(object)
                })
        }

        pub async fn put_deployment(
            &mut self,
            deployment: &K8sObject<Deployment>,
        ) -> Result<K8sObject<Deployment>, K8sClientError> {
            let name = deployment.metadata.name.clone().unwrap();
            let namespace = deployment.metadata.namespace.clone().unwrap();
            self.put(
                deployment,
                namespace.as_str(),
                "deployments",
                "apps",
                "v1",
                name.as_str(),
            )
            .await
        }

        // Create or Update
        pub async fn save_deployment(
            &mut self,
            deployment: &K8sObject<Deployment>,
        ) -> Result<K8sObject<Deployment>, K8sClientError> {
            let result = self.post_deployment(deployment).await;
            match result {
                Ok(_) => result,
                Err(Conflict) => {
                    info!(
                        "Deployment {} already exists, updating",
                        deployment.metadata.name.clone().unwrap()
                    );
                    self.put_deployment(deployment).await
                }
                Err(e) => Err(e),
            }
        }

        pub async fn put_service(
            &mut self,
            service: &K8sObject<Service>,
        ) -> Result<K8sObject<Service>, K8sClientError> {
            let name = service.metadata.name.clone().unwrap();
            let namespace = service.metadata.namespace.clone().unwrap();
            let url = format!(
                "{}/api/v1/namespaces/{}/services/{}",
                self.get_api_server_url(),
                namespace,
                name,
            );
            self.put_with_url(service, url.as_str()).await
        }

        pub async fn post_deployment(
            &mut self,
            deployment: &K8sObject<Deployment>,
        ) -> Result<K8sObject<Deployment>, K8sClientError> {
            let namespace = deployment.metadata.namespace.clone().unwrap();
            self.post(deployment, namespace.as_str(), "deployments", "apps", "v1")
                .await
        }

        pub async fn watch<T: DeserializeOwned>(
            &mut self,
            uri: &str,
            resource_version: &str,
        ) -> Result<impl Stream<Item = Watch<K8sListObject<T>>>, K8sClientError> {
            let mut response = self
                .send_with_retry(self.client.get(format!(
                    "{}/{}?watch=1&resourceVersion={}",
                    self.get_api_server_url(),
                    uri,
                    resource_version
                )))
                .await;
            let status = response.status();
            if let Some(error) = K8sClientError::from_status(status, "") {
                return Err(error);
            }
            Ok(stream! {
                loop {
                    if let Some(chunk) = response.chunk().await.unwrap() {
                        let payload = from_utf8(chunk.as_ref()).unwrap();
                        match from_str::<Watch<K8sListObject<T>>>(payload) {
                            Ok(event) => {
                                yield event;
                            }
                            Err(e) => {
                                error!("Error occurred while trying to watch k8s object: {:?}", e);
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            })
        }

        pub async fn watch_exposed_apps(
            &mut self,
            resource_version: &str,
        ) -> Result<impl Stream<Item = Watch<K8sListObject<ExposedApp>>>, K8sClientError> {
            self.watch(EXPOSED_APPS_LIST, resource_version).await
        }

        pub async fn get_all<T: DeserializeOwned>(
            &mut self,
            uri: &str,
        ) -> Result<List<K8sListObject<T>>, K8sClientError> {
            let response = self
                .send_with_retry(
                    self.client
                        .get(format!("{}/{}", self.get_api_server_url(), uri)),
                )
                .await;
            let status = response.status();
            let text = response.text().await.unwrap();
            K8sClientError::from_status(status, text.as_str())
                .map(Err)
                .unwrap_or_else(|| {
                    let object = from_str::<List<K8sListObject<T>>>(text.as_str()).unwrap();
                    Ok(object)
                })
        }

        pub async fn get_lease(
            &mut self,
            namespace: &str,
            name: &str,
        ) -> Result<K8sObject<Lease>, K8sClientError> {
            let url = format!(
                "{}/apis/coordination.k8s.io/v1/namespaces/{}/leases/{}",
                self.get_api_server_url(),
                namespace,
                name
            );
            let result = self.send_with_retry(self.client.get(url)).await;
            let status = result.status();
            let text = result.text().await.unwrap();
            K8sClientError::from_status(status, text.as_str())
                .map(Err)
                .unwrap_or_else(|| {
                    let lease = from_str::<K8sObject<Lease>>(text.as_str()).unwrap();
                    Ok(lease)
                })
        }

        pub async fn put_exposed_app_status(
            &mut self,
            namespace: &str,
            name: &str,
            app: &K8sObject<ExposedApp>,
        ) -> Result<K8sObject<ExposedApp>, K8sClientError> {
            let url = format!(
                "{}/apis/stable.no-library.com/v1/namespaces/{}/exposedapps/{}/status",
                self.get_api_server_url(),
                namespace,
                name
            );
            self.execute(self.client.put(url), app).await
        }

        pub async fn post_event(
            &mut self,
            namespace: &str,
            event: &K8sObject<Event>,
        ) -> Result<K8sObject<Event>, K8sClientError> {
            let url = format!(
                "{}/apis/events.k8s.io/v1/namespaces/{}/events",
                self.get_api_server_url(),
                namespace
            );
            self.execute(self.client.post(url), event).await
        }

        pub async fn patch_lease(
            &mut self,
            namespace: &str,
            name: &str,
            resource_version: &str,
            holder_identity: &str,
            acquire_time: OffsetDateTime,
        ) -> Result<(), K8sClientError> {
            let entries = vec![
                JsonPatchEntry {
                    op: String::from("test"),
                    path: String::from("/metadata/resourceVersion"),
                    value: String::from(resource_version),
                },
                JsonPatchEntry {
                    op: String::from("add"),
                    path: String::from("/spec/holderIdentity"),
                    value: String::from(holder_identity),
                },
                JsonPatchEntry {
                    op: String::from("add"),
                    path: String::from("/spec/acquireTime"),
                    value: format(acquire_time).unwrap(),
                },
            ];
            let serialized = to_string(&entries).unwrap();
            let url = format!(
                "{}/apis/coordination.k8s.io/v1/namespaces/{}/leases/{}",
                self.get_api_server_url(),
                namespace,
                name
            );
            let response = self
                .send_with_retry(
                    self.client
                        .patch(url)
                        .body(serialized)
                        .header("Content-Type", "application/json-patch+json"),
                )
                .await;
            let status = response.status();
            let text = response.text().await.unwrap();
            K8sClientError::from_status(status, text.as_str())
                .map(Err)
                .unwrap_or(Ok(()))
        }
    }
}
