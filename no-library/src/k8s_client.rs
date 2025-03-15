pub mod client {
    use crate::k8s_client::client::K8sClientError::{Conflict, Error, NotFound};
    use crate::k8s_types::{Deployment, ExposedApp, K8sObject, List, Service, Watch};
    use async_stream::stream;
    use futures::Stream;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::{Certificate, Client, RequestBuilder, Response, StatusCode};
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::{from_str, to_string};
    use std::str::from_utf8;
    use std::time::Duration;
    use tokio::fs;
    use tokio::time::sleep;
    use tracing::{error, info};

    const SERVICE_ACCOUNT_PATH: &str = "/var/run/secrets/kubernetes.io/serviceaccount";
    const API_SERVER: &str = "https://kubernetes.default.svc";
    const EXPOSED_APPS_LIST: &str = "/apis/stable.no-library.com/v1/exposedapps";

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

    #[derive(Debug, Clone, Copy)]
    pub enum K8sClientError {
        NotFound,
        Conflict,
        Error,
    }

    impl K8sClientError {
        pub fn from_status(status: StatusCode) -> Option<K8sClientError> {
            match status.as_u16() {
                404 => Some(NotFound),
                409 => Some(Conflict),
                _ => {
                    if !status.is_success() {
                        Some(Error)
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
    }

    impl K8sClient {
        pub async fn new() -> Self {
            K8sClient {
                client: get_client().await,
                token: get_token().await,
            }
        }

        fn get_auth_header(&self) -> HeaderMap {
            let mut headers = HeaderMap::new();
            let value = HeaderValue::from_str(format!("Bearer {}", self.token).as_str()).unwrap();
            headers.insert("Authorization", value);
            headers
        }

        pub async fn get_exposed_apps(&mut self) -> List<K8sObject<ExposedApp>> {
            let result = self
                .send_with_retry(
                    self.client
                        .get(format!("{}{}", API_SERVER, EXPOSED_APPS_LIST)),
                )
                .await;
            from_str::<List<K8sObject<ExposedApp>>>(result.text().await.unwrap().as_str()).unwrap()
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
                API_SERVER, group, version, namespace, resource_type, name
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
                API_SERVER, group, version, namespace, resource_type,
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
                .headers(self.get_auth_header())
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
                    .headers(self.get_auth_header())
                    .send()
                    .await
                    .unwrap();
                last_status = response.status().as_u16();
                result = response;
                retries = retries - 1;
            }
            result
        }

        async fn execute<T: Serialize + DeserializeOwned>(
            &mut self,
            builder: RequestBuilder,
            item: &K8sObject<T>,
        ) -> Result<K8sObject<T>, K8sClientError> {
            let payload = to_string(&item).unwrap();
            let result = self.send_with_retry(builder.body(payload)).await;
            let status = result.status();
            let text = result.text().await.unwrap();
            K8sClientError::from_status(status)
                .map(Err)
                .unwrap_or_else(|| {
                    let object = from_str::<K8sObject<T>>(text.as_str()).unwrap();
                    Ok(object)
                })
        }

        pub async fn put_exposed_app(
            &mut self,
            apps: &K8sObject<ExposedApp>,
        ) -> Result<K8sObject<ExposedApp>, K8sClientError> {
            let name = apps.metadata.name.clone().unwrap();
            let namespace = apps.metadata.namespace.clone().unwrap();
            self.put(
                apps,
                namespace.as_str(),
                "exposedapps",
                "stable.no-library.com",
                "v1",
                name.as_str(),
            )
            .await
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

        pub async fn put_service(
            &mut self,
            service: &K8sObject<Service>,
        ) -> Result<K8sObject<Service>, K8sClientError> {
            let name = service.metadata.name.clone().unwrap();
            let namespace = service.metadata.namespace.clone().unwrap();
            let url = format!(
                "{}/api/v1/namespaces/{}/services/{}",
                API_SERVER, namespace, name,
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

        pub async fn delete(
            &mut self,
            namespace: &str,
            resource_type: &str,
            group: &str,
            version: &str,
            name: &str,
        ) -> Result<(), K8sClientError> {
            let uri = format!(
                "/apis/{}{}/namespaces/{}/{}/{}",
                group, version, namespace, resource_type, name
            );
            self.delete_uri(uri.as_str()).await
        }

        pub async fn delete_uri(&mut self, uri: &str) -> Result<(), K8sClientError> {
            let url = format!("{}{}", API_SERVER, uri);
            let response = self.send_with_retry(self.client.delete(url)).await;
            K8sClientError::from_status(response.status())
                .map(Err)
                .unwrap_or(Ok(()))
        }

        pub async fn watch_exposed_apps(
            &mut self,
            resource_version: &str,
        ) -> impl Stream<Item = Watch<K8sObject<ExposedApp>>> {
            let mut response = self
                .send_with_retry(self.client.get(format!(
                    "{}{}?watch=1&resourceVersion={}",
                    API_SERVER, EXPOSED_APPS_LIST, resource_version
                )))
                .await;
            stream! {
                loop {
                    if let Some(chunk) = response.chunk().await.unwrap() {
                        let event = from_str::<Watch<K8sObject<ExposedApp>>>(from_utf8(chunk.as_ref()).unwrap()).unwrap();
                        yield event;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
