use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use anyhow::{Context, Error};
use reqwest::{Client, Response};
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

pub struct ApiClient {
    inner: Arc<Client>,
    base_url: String,
    timeout: Duration,
    max_retries: usize
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>, timeout: Duration, max_retries: usize) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client build");

        Self {
            inner: Arc::new(client),
            base_url: base_url.into(),
            timeout,
            max_retries
        }
    }


    pub async fn get_subaccount_info(&self, worker_full_name: String) -> anyhow::Result<SubAccountInfo> {
        let url = format!("{}/users/get-subAccount-info?workerName={}", self.base_url, worker_full_name);

        let mut attempt = 0;
        loop {
            attempt += 1;
            let res = self
                .inner
                .get(&url)
                .send()
                .await
                .context("http request field");

            match res {
                Ok(r) => {
                    if r.status().is_success() {
                        let text = r.text().await.context("read body")?;
                        info!("text -> {:?}", text);
                        let info = serde_json::from_str(&text)
                            .context("deserialize error")?;

                        return Ok(info);
                    } else if r.status().is_client_error() {
                        anyhow::bail!("Client error: {}", r.status());
                    } else {
                        if attempt > self.max_retries {
                            anyhow::bail!("server error after retries: {}", r.status());
                        }
                    }
                }
                Err(e) => {
                    if attempt > self.max_retries {
                        return Err(e).context("request attempts exhausted");
                    }
                }
            }

            let backoff = Duration::from_millis(50 * (attempt as u64).pow(2));
            tokio::time::sleep(backoff).await;
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SubAccountInfo {
    pub id: String,
    #[serde(rename = "minerId")]
    pub miner_id: String,
    #[serde(rename = "poolTarget")]
    pub pool_target: String,
    #[serde(rename = "subAccountName")]
    pub sub_account_name: String,
    pub active: bool,
    pub metadata: Value,
    #[serde(rename = "createdAt")]
    pub created_at: String
}