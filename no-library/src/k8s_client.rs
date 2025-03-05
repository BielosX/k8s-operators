pub mod client {
    use reqwest::{Certificate, Client};
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
        let certificate = Certificate::from_pem(cert.as_slice()).expect("Unable to create Certificate");
        Client::builder()
            .add_root_certificate(certificate)
            .build()
            .expect("Unable to add root certificate")
    }

    pub struct K8sClient {
        client: Client
    }

    impl K8sClient {
        pub async fn new() -> Self {
            K8sClient {
                client: get_client().await
            }
        }

        pub async fn get_exposed_apps(&self) -> String {
            let token = get_token().await;
            let request = self.client
                .get(format!("{}{}", API_SERVER, EXPOSED_APPS_LIST))
                .header("Authorization", format!("Bearer {}", token))
                .build()
                .unwrap();
            let response = self.client
                .execute(request)
                .await
                .unwrap();
            response.text().await.unwrap()
        }
    }
}