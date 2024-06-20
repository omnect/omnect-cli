use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use url::Url;

use crate::cli::EcosConfig;
use crate::identity::PkiProvider;

lazy_static::lazy_static! {
    static ref PKI_API: Url = {
        let host = std::env::var("HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        let tenant = std::env::var("TENANT").ok();

        match tenant {
            Some(tenant) => Url::parse(
                &format!("https://{host}/{tenant}")
            ).unwrap(),
            None => Url::parse(
                &format!("https://{host}")
            ).unwrap(),
        }

    };
}

const AUTH_ENDPOINT: &str = "/_auth";

#[allow(dead_code)]
const CONTENT_TYPE: &str = "application/vnd.api+json";

#[derive(Clone)]
#[allow(dead_code)]
pub struct Password(String);

impl From<&str> for Password {
    fn from(source: &str) -> Self {
        Self(source.to_string())
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Password <redacted>")
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CaId(String);

impl From<&str> for CaId {
    fn from(ca_id: &str) -> Self {
        Self(ca_id.to_string())
    }
}

#[derive(Deserialize)]
struct AuthReply {
    result: String,
    token: String,
}

#[allow(dead_code)]
pub struct EcosBackend {
    client: reqwest::Client,
    bearer: String,
    ca: CaId,
}

impl EcosBackend {
    pub async fn try_from(config: EcosConfig) -> Result<EcosBackend> {
        let client = reqwest::Client::new();

        let response: AuthReply = client
            .post(PKI_API.join(AUTH_ENDPOINT)?)
            .json(&json!({
                "uid": config.user,
                "pw": config.password.0,
            }))
            .send()
            .await?
            .json()
            .await?;

        anyhow::ensure!(response.result == "true");

        Ok(EcosBackend {
            client,
            bearer: response.token,
            ca: config.ca,
        })
    }
}

#[async_trait::async_trait]
impl PkiProvider for EcosBackend {
    async fn sign_csr(&self, _csr: openssl::x509::X509Req) -> Result<openssl::x509::X509> {
        // let response = self.client.
        // .get(PKI_API.join(SOME_ENDPOINT))
        // .bearer_auth(self.bearer)
        // ...

        // cert_ca or cert_server?
        // TODO: step 1: PUT cert_ca object w/ CSR
        // - What data is necessary
        // - Refer to existing CA?
        // TODO: step 2: PUT CSR
        // TODO: step 3: Download Cert
        // - Where do we get the link from?
        // TODO: step 4: retrieve cert chain
        unimplemented!()
    }

    async fn full_chain_cert(&self) -> Result<openssl::x509::X509> {
        // TODO fetch cert from API
        unimplemented!()
    }
}
