use actix_web::web::post;
use anyhow::Result;
use log::debug;
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

const AUTH_ENDPOINT: &str = "/bbmaindb/_auth";
const CERT_SERVER_ENDPOINT: &str = "/api/v2.0/cert_server";
const CERT_SERVER_CA_ID: &str = "d3486bb63091f72d86b3834f2a02eb80";

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
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        let params = [("uid", config.user), ("pw", config.password.0)];

        let response: AuthReply = client
            .post(PKI_API.join(AUTH_ENDPOINT)?)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        anyhow::ensure!(response.result == "ok");

        debug!("{}", response.token);

        Ok(EcosBackend {
            client,
            bearer: response.token,
            ca: config.ca,
        })
    }
}

#[async_trait::async_trait]
impl PkiProvider for EcosBackend {
    async fn sign_csr(&self, csr: openssl::x509::X509Req) -> Result<openssl::x509::X509> {
        debug!("csr: {:?}", &csr.to_text());
        let device_id = csr
            .subject_name()
            .entries()
            .map(|e| e.data().as_utf8().unwrap().to_string());
        let device_id = device_id.collect::<Vec<String>>();
        let device_id = device_id.join(",");

        let response = self
            .client
            .post(PKI_API.join(CERT_SERVER_ENDPOINT)?)
            .bearer_auth(&self.bearer)
            .query(&[("minfields", "cert_csr_file")])
            .json(&json!({
               "data" : {
                  "type" : "cert_server",
                  "attributes" : {
                      "cn": device_id,
                      "cert_key_crypt": "never",
                      "cert_key_size": 4096
                  },
                  "relationships" : {
                      "cert_ca" : {
                          "data" : {
                           "id" : CERT_SERVER_CA_ID,
                           "type" : "cert_ca"
                        }
                      }
                  }
               }
            }))
            .send()
            .await?;

        anyhow::ensure!(response.status().is_success());

        let response: serde_json::Value = serde_json::from_str(&response.text().await?)?;

        debug!("create cert: {:#?}", response);

        let csr_endpoint =
            &response["data"]["attributes"]["cert_csr_file"]["data"]["links"]["attachment"];

        debug!("csr endpoint{}", csr_endpoint);

        let response = self
            .client
            .put(PKI_API.join(&csr_endpoint.as_str().unwrap())?)
            .bearer_auth(&self.bearer)
            .body(csr.to_der()?)
            .send()
            .await?;

        debug!("{:#?}", response);

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
