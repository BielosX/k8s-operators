pub mod client {
    use crate::k8s_types::{ExposedApp, List, Watch};
    use async_stream::stream;
    use futures::Stream;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::{Certificate, Client};
    use std::str::from_utf8;
    use tokio::fs;

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

        pub async fn get_exposed_apps(&self) -> List<ExposedApp> {
            let result = self
                .client
                .get(format!("{}{}", API_SERVER, EXPOSED_APPS_LIST))
                .headers(self.get_auth_header())
                .send()
                .await
                .unwrap();
            serde_json::from_str::<List<ExposedApp>>(result.text().await.unwrap().as_str()).unwrap()
        }

        pub async fn watch_exposed_apps(
            &self,
            resource_version: &str,
        ) -> impl Stream<Item = Watch<ExposedApp>> {
            let mut response = self
                .client
                .get(format!(
                    "{}{}?watch=1&resourceVersion={}",
                    API_SERVER, EXPOSED_APPS_LIST, resource_version
                ))
                .headers(self.get_auth_header())
                .send()
                .await
                .unwrap();
            stream! {
                loop {
                    if let Some(chunk) = response.chunk().await.unwrap() {
                        let event = serde_json::from_str::<Watch<ExposedApp>>(from_utf8(chunk.as_ref()).unwrap()).unwrap();
                        yield event;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
