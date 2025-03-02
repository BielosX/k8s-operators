use tokio::time::{sleep, Duration};
use tracing::info;

async fn get_ip() -> reqwest::Result<String> {
    reqwest::get("https://api.ipify.org")
        .await?
        .text()
        .await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    loop {
        let ip = get_ip().await.unwrap();
        info!("Public IP: {}", ip);
        sleep(Duration::from_secs(10)).await;
    }
}
