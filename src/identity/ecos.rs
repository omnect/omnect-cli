use anyhow::Result;
use url::Url;

use crate::cli::EcosConfig;
use crate::identity::PkiProvider;

const API_VERSION: &str = "v2.0";

lazy_static::lazy_static! {
    static ref PKI_API: Url = {
        let host = std::env::var("HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        let tenant = std::env::var("TENANT").ok();

        match tenant {
            Some(tenant) => Url::parse(
                &format!("https://{host}/{tenant}/api/{API_VERSION}/")
            ).unwrap(),
            None => Url::parse(
                &format!("https://{host}/api/{API_VERSION}/")
            ).unwrap(),
        }

    };
}

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

#[async_trait::async_trait]
impl PkiProvider for EcosConfig {
    async fn sign_csr(&self, _csr: openssl::x509::X509Req) -> Result<openssl::x509::X509> {
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
        unimplemented!()
    }
}
