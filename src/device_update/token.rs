use anyhow::{Context, Result};
use log::debug;
use oauth2::{ClientId, ClientSecret, Scope, TokenResponse, TokenUrl};
use tokio::sync::Mutex;

const AZURE_AD_TOKEN_URL_PREFIX: &str = "https://login.microsoftonline.com/";
const AZURE_AD_TOKEN_URL_SUFFIX: &str = "/oauth2/v2.0/token";
const ADU_API_SCOPE: &str = "https://api.adu.microsoft.com/.default";
// Refresh the token 60 seconds before actual expiry to avoid races
const TOKEN_EXPIRY_MARGIN_SECS: u64 = 60;
const TOKEN_REQUEST_TIMEOUT_SECS: u64 = 30;

struct CachedToken {
    access_token: String,
    expires_at: std::time::Instant,
}

pub(crate) struct AzureTokenProvider {
    client_id: ClientId,
    client_secret: ClientSecret,
    token_url: TokenUrl,
    http_client: oauth2::reqwest::Client,
    scope: Scope,
    cached: Mutex<Option<CachedToken>>,
}

impl AzureTokenProvider {
    pub(crate) fn new(tenant_id: &str, client_id: &str, client_secret: &str) -> Result<Self> {
        let token_url_str =
            format!("{AZURE_AD_TOKEN_URL_PREFIX}{tenant_id}{AZURE_AD_TOKEN_URL_SUFFIX}");

        let http_client = oauth2::reqwest::ClientBuilder::new()
            .redirect(oauth2::reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(TOKEN_REQUEST_TIMEOUT_SECS))
            .build()
            .context("failed to create HTTP client for token provider")?;

        Ok(Self {
            client_id: ClientId::new(client_id.to_string()),
            client_secret: ClientSecret::new(client_secret.to_string()),
            token_url: TokenUrl::new(token_url_str).context("invalid Azure AD token URL")?,
            http_client,
            scope: Scope::new(ADU_API_SCOPE.to_string()),
            cached: Mutex::new(None),
        })
    }

    pub(crate) async fn get_token(&self) -> Result<String> {
        let mut cached = self.cached.lock().await;

        if let Some(ref token) = *cached {
            if token.expires_at > std::time::Instant::now() {
                debug!("using cached Azure AD token");
                return Ok(token.access_token.clone());
            }
            debug!("cached Azure AD token expired, refreshing");
        }

        let client = oauth2::basic::BasicClient::new(self.client_id.clone())
            .set_client_secret(self.client_secret.clone())
            .set_token_uri(self.token_url.clone());

        let response = client
            .exchange_client_credentials()
            .add_scope(self.scope.clone())
            .request_async(&self.http_client)
            .await
            .context("failed to acquire Azure AD token via client credentials")?;

        let access_token = response.access_token().secret().to_string();

        let expires_in = response
            .expires_in()
            .unwrap_or(std::time::Duration::from_secs(3600));

        let expires_at = std::time::Instant::now()
            + expires_in.saturating_sub(std::time::Duration::from_secs(TOKEN_EXPIRY_MARGIN_SECS));

        debug!(
            "acquired Azure AD token, expires in {}s",
            expires_in.as_secs()
        );

        *cached = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at,
        });

        Ok(access_token)
    }
}
