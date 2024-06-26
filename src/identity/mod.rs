use anyhow::{Context, Result};
use async_trait::async_trait;

use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;

pub mod ecos;

use crate::cli::ProviderConfig;

fn create_key_pair() -> Result<PKey<Private>> {
    let rsa = Rsa::generate(4096)?;
    Ok(PKey::from_rsa(rsa)?)
}

fn create_csr(device_id: &str, pkey: &PKey<Private>) -> Result<openssl::x509::X509Req> {
    let mut subject_name_builder = openssl::x509::X509NameBuilder::new()?;
    subject_name_builder.append_entry_by_text("C", "DE")?;
    subject_name_builder.append_entry_by_text("ST", "BY")?;
    subject_name_builder.append_entry_by_text("O", "conplement AG")?;
    subject_name_builder.append_entry_by_text("CN", device_id)?;
    let subject_name = subject_name_builder.build();

    let mut exts = openssl::stack::Stack::new()?;
    exts.push(
        openssl::x509::extension::ExtendedKeyUsage::new()
            .client_auth()
            .build()?,
    )?;

    let mut csr_req_builder = openssl::x509::X509Req::builder()?;
    csr_req_builder.set_version(0)?;
    csr_req_builder.set_subject_name(&subject_name)?;
    csr_req_builder.add_extensions(&exts)?;
    csr_req_builder.set_pubkey(pkey)?;
    csr_req_builder.sign(pkey, openssl::hash::MessageDigest::sha256())?;

    Ok(csr_req_builder.build())
}

pub struct Certs {
    pub full_chain_cert: Vec<u8>,
    pub device_cert: Vec<u8>,
    pub device_key: Vec<u8>,
}

pub async fn request_cert_from_pki(device_id: &str, provider: impl PkiProvider) -> Result<Certs> {
    let pkey = create_key_pair().context("couldn't create key pair")?;
    let csr = create_csr(device_id, &pkey).context("couldn't create CSR")?;

    //#[tokio::main]
    async fn request_certs(
        provider: impl PkiProvider,
        csr: openssl::x509::X509Req,
    ) -> Result<(openssl::x509::X509, String)> {
        let (device_cert, full_chain_cert) = provider
            .sign_csr(csr)
            .await
            .context("couldn't retrieve certificate")?;
        // let full_chain_cert = provider
        //     .full_chain_cert()
        //     .await
        //     .context("couldn't get full chain certificate")?;

        Ok((device_cert, full_chain_cert))
    }

    let (device_cert, full_chain_cert) = request_certs(provider, csr)
        .await
        .context("couldnt't request certificates from pki provider")?;

    let intermediate_full_chain_cert = full_chain_cert;

    // let intermediate_full_chain_cert = full_chain_cert
    //     .to_pem()
    //     .context("couldn't serialize full chain certificate")?;
    let device_cert_pem = device_cert
        .to_pem()
        .context("couldn't serialize certificate")?;
    let device_key_pem = pkey
        .private_key_to_pem_pkcs8()
        .context("couldn't serialize key")?;

    Ok(Certs {
        full_chain_cert: intermediate_full_chain_cert.into(),
        device_cert: device_cert_pem,
        device_key: device_key_pem,
    })
}

#[async_trait]
pub trait PkiProvider {
    async fn sign_csr(&self, csr: openssl::x509::X509Req) -> Result<(openssl::x509::X509, String)>;

    // async fn full_chain_cert(&self) -> Result<openssl::x509::X509>;
}

pub async fn create_est_backend(backend_config: ProviderConfig) -> Result<impl PkiProvider> {
    match backend_config {
        ProviderConfig::Ecos(config) => ecos::EcosBackend::try_from(config).await,
    }
}
