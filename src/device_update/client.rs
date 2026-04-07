use anyhow::{Context, Result, bail};
use log::{debug, info, warn};
use serde::Deserialize;
use url::Url;

use crate::device_update::token::AzureTokenProvider;

const API_VERSION: &str = "2022-10-01";
const INITIAL_POLL_INTERVAL_SECS: u64 = 2;
const MAX_POLL_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateOperation {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: OperationStatus,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub(crate) enum OperationStatus {
    NotStarted,
    Running,
    Succeeded,
    Failed,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
            Self::Running => write!(f, "Running"),
            Self::Succeeded => write!(f, "Succeeded"),
            Self::Failed => write!(f, "Failed"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

pub(crate) struct DeviceUpdateClient {
    endpoint: Url,
    http_client: reqwest::Client,
    token_provider: AzureTokenProvider,
}

impl DeviceUpdateClient {
    pub(crate) fn new(
        endpoint: &str,
        tenant_id: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<Self> {
        let endpoint = Url::parse(endpoint).context("invalid device update endpoint URL")?;
        let token_provider = AzureTokenProvider::new(tenant_id, client_id, client_secret)?;
        let http_client = reqwest::Client::new();

        Ok(Self {
            endpoint,
            http_client,
            token_provider,
        })
    }

    pub(crate) async fn import_update(
        &self,
        instance_id: &str,
        import_json: String,
    ) -> Result<UpdateOperation> {
        let url = self
            .endpoint
            .join(&format!(
                "/deviceupdate/{instance_id}/updates:import?api-version={API_VERSION}"
            ))
            .context("failed to construct import URL")?;

        debug!("POST {url}");

        let token = self.token_provider.get_token().await?;

        let response = self
            .http_client
            .post(url.as_str())
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .body(import_json)
            .send()
            .await
            .context("import update request failed")?;

        self.handle_async_response(response, "import update").await
    }

    pub(crate) async fn delete_update(
        &self,
        instance_id: &str,
        provider: &str,
        name: &str,
        version: &str,
    ) -> Result<UpdateOperation> {
        let url = self
            .endpoint
            .join(&format!(
                "/deviceupdate/{instance_id}/updates/providers/{provider}/names/{name}/versions/{version}?api-version={API_VERSION}"
            ))
            .context("failed to construct delete URL")?;

        debug!("DELETE {url}");

        let token = self.token_provider.get_token().await?;

        let response = self
            .http_client
            .delete(url.as_str())
            .bearer_auth(&token)
            .send()
            .await
            .context("delete update request failed")?;

        self.handle_async_response(response, "delete update").await
    }

    async fn handle_async_response(
        &self,
        response: reqwest::Response,
        operation_name: &str,
    ) -> Result<UpdateOperation> {
        let status = response.status();

        match status.as_u16() {
            200 => {
                let body = response
                    .text()
                    .await
                    .context("failed to read response body")?;
                serde_json::from_str(&body)
                    .context("failed to parse immediate response as UpdateOperation")
            }
            202 => {
                let operation_location = response
                    .headers()
                    .get("operation-location")
                    .context("202 response missing operation-location header")?
                    .to_str()
                    .context("operation-location header is not valid UTF-8")?
                    .to_string();

                info!("{operation_name}: accepted, polling {operation_location}");

                self.poll_operation(&operation_location).await
            }
            _ => {
                let body = response.text().await.unwrap_or_default();
                bail!("{operation_name} failed with status {status}: {body}");
            }
        }
    }

    async fn poll_operation(&self, operation_url: &str) -> Result<UpdateOperation> {
        let poll_url = if operation_url.starts_with("http") {
            Url::parse(operation_url).context("invalid operation-location URL")?
        } else {
            self.endpoint
                .join(operation_url)
                .context("failed to resolve relative operation-location URL")?
        };

        let mut interval_secs = INITIAL_POLL_INTERVAL_SECS;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

            let token = self.token_provider.get_token().await?;

            let response = self
                .http_client
                .get(poll_url.as_str())
                .bearer_auth(&token)
                .send()
                .await
                .context("failed to poll operation status")?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                bail!("operation status poll failed with status {status}: {body}");
            }

            let body = response
                .text()
                .await
                .context("failed to read operation status response")?;

            let operation: UpdateOperation =
                serde_json::from_str(&body).context("failed to parse operation status response")?;

            debug!(
                "operation {} status: {}",
                operation.operation_id, operation.status
            );

            match operation.status {
                OperationStatus::Succeeded => return Ok(operation),
                OperationStatus::Failed => {
                    let error_detail = operation
                        .error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "no error details".to_string());
                    bail!(
                        "operation {} failed: {error_detail}",
                        operation.operation_id
                    );
                }
                OperationStatus::Unknown => {
                    warn!(
                        "operation {} returned unknown status, continuing to poll",
                        operation.operation_id
                    );
                }
                _ => {}
            }

            interval_secs = (interval_secs * 2).min(MAX_POLL_INTERVAL_SECS);
        }
    }
}
