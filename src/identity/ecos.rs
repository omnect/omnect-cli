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
// TODO don't use the static ID but use it from the CLI
const CERT_SERVER_CA_ID: &str = "d3486bb63091f72d86b3834f2a02eb80";

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

enum Endpoint {
    Csr,
    Der,
    CaId,
    CaType,
    Pem,
    Sign,
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Endpoint::Csr => write!(f, "CSR"),
            Endpoint::Der => write!(f, "DER"),
            Endpoint::CaId => write!(f, "CA Id"),
            Endpoint::CaType => write!(f, "CA Type"),
            Endpoint::Pem => write!(f, "PEM"),
            Endpoint::Sign => write!(f, "Sign"),
        }
    }
}

fn endpoint(endpoint: Endpoint, response: &serde_json::Value) -> Result<&str> {
    let path = match endpoint {
        Endpoint::Csr => "/data/attributes/cert_csr_file/data/links/attachment",
        Endpoint::Der => "/data/attributes/cert_crt_file/data/links/attachment",
        Endpoint::CaId => "/data/relationships/cert_ca/data/id",
        Endpoint::CaType => "/data/relationships/cert_ca/data/type",
        Endpoint::Pem => "/data/attributes/cert_crt_file/data/links/attachment_pem",
        Endpoint::Sign => "/links/sign/href",
    };

    let elem = response.pointer(path).ok_or(anyhow::anyhow!(
        "Could not retrieve endpoint \"{endpoint}\" from request",
    ))?;

    Ok(elem.as_str().unwrap())
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
    async fn sign_csr(&self, csr: openssl::x509::X509Req) -> Result<(openssl::x509::X509, String)> {
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
            .query(&[("minfields", "cert_csr_file,cert_crt_file")])
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

        let response: serde_json::Value = response.json().await?;

        // TODO we must add the certificate to the correct certificate subscriber, here, as well.

        debug!("create cert: {:#?}", response);

        let csr_endpoint = endpoint(Endpoint::Csr, &response)?;
        let der_endpoint = endpoint(Endpoint::Der, &response)?;
        let mut cert_ca_id = endpoint(Endpoint::CaId, &response)?.to_string();
        let mut cert_ca_type = endpoint(Endpoint::CaType, &response)?.to_string();
        let cert_id = &response["data"]["id"];

        debug!("csr endpoint{}", csr_endpoint);

        let response = self
            .client
            .put(PKI_API.join(csr_endpoint)?)
            .bearer_auth(&self.bearer)
            .body(csr.to_der()?)
            .send()
            .await?;

        debug!("{:#?}", response);

        let response: serde_json::Value = response.json().await?;

        debug!("{:#?}", response);

        let response = self
            .client
            .get(PKI_API.join(&format!(
                "{}/{}",
                CERT_SERVER_ENDPOINT,
                cert_id.as_str().unwrap()
            ))?)
            .bearer_auth(&self.bearer)
            .send()
            .await?;

        debug!("{:#?}", response);
        let response: serde_json::Value = response.json().await.unwrap();
        debug!("{:#?}", response);
        let sign_endpoint = endpoint(Endpoint::Sign, &response)?;

        debug!("{:#?}", &self.bearer);

        let response = self
            .client
            .post(PKI_API.join(sign_endpoint)?)
            .bearer_auth(&self.bearer)
            .json(&json!({}))
            .send()
            .await;

        debug!("Sign response: {:#?}", response);

        let response = response.unwrap();

        debug!("signd endpoint {:#?}", response);

        let response: serde_json::Value = response.json().await?;

        debug!("signd endpoint {:#?}", response);

        let response = self
            .client
            .get(PKI_API.join(der_endpoint)?)
            .bearer_auth(&self.bearer)
            .send()
            .await?;

        let device_cert = openssl::x509::X509::from_der(&response.bytes().await?)?;

        // get full chain cert

        // TODO probably it is best to factor this operation out to the
        // full_chain_cert function. therefore this function must be uncommented
        // in the trait definition and then called accordingly.

        let mut chain_pems: Vec<String> = vec![];

        loop {
            let response = self
                .client
                .get(PKI_API.join(&format!("/api/v2.0/{}/{}", cert_ca_type, cert_ca_id))?)
                .bearer_auth(&self.bearer)
                .send()
                .await?;

            debug!("{:#?}", response);
            let response: serde_json::Value = response.json().await.unwrap();
            debug!("{:#?}", &response);

            cert_ca_id = endpoint(Endpoint::CaId, &response)
                .unwrap_or_default()
                .to_string();
            cert_ca_type = endpoint(Endpoint::CaType, &response)
                .unwrap_or_default()
                .to_string();

            let pem_endpoint = endpoint(Endpoint::Pem, &response)?;

            let response = self
                .client
                .get(PKI_API.join(pem_endpoint)?)
                .bearer_auth(&self.bearer)
                .send()
                .await?;

            chain_pems.push(response.text().await.unwrap());

            if cert_ca_id.is_empty() {
                break;
            }
        }

        let chain_cert = chain_pems.join("");

        debug!("Chain cert:\n{chain_cert}");
        debug!("Device Cert:\n{device_cert:#?}");

        Ok((device_cert, chain_cert))
    }

    // async fn full_chain_cert(&self) -> Result<openssl::x509::X509> {
    //     // TODO fetch cert from API
    //     unimplemented!()
    // }
}
