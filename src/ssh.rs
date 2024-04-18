use std::fs;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;

use anyhow::Result;
use directories::ProjectDirs;
use oauth2::AccessToken;
use serde::{Deserialize, Serialize};
use url::Url;

static BACKEND_API_ENDPOINT: &str = "/api/devices/prepareSSHConnection";
static SSH_KEY_FORMAT: &str = "ed25519";

static BASTION_CERT_NAME: &str = "bastion-cert.pub";
static DEVICE_CERT_NAME: &str = "device-cert.pub";
static SSH_CONFIG_NAME: &str = "config";

pub struct Config {
    backend: Url,
    dir: PathBuf,
    priv_key_path: Option<PathBuf>,
    config_path: PathBuf,
}

impl Config {
    pub fn new(
        backend: impl AsRef<str>,
        dir: Option<PathBuf>,
        priv_key_path: Option<PathBuf>,
        config_path: Option<PathBuf>,
    ) -> Result<Config> {
        let backend = match Url::parse(backend.as_ref()) {
            Ok(url) => url,
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                // url has no scheme, attempt to fix it
                Url::parse(&format!("https://{}", backend.as_ref())).map_err(|_| {
                    anyhow::anyhow!("Invalid backend url: \"{}\".", backend.as_ref())
                })?
            }
            Err(_) => {
                anyhow::bail!("Invalid backend url: \"{}\".", backend.as_ref())
            }
        };

        let dir = match dir {
            Some(dir) => {
                if let Ok("true") | Ok("1") = std::env::var("CONTAINERIZED").as_deref() {
                    anyhow::bail!(
                        "Custom config paths are not supported in containerized environments."
                    );
                }

                dir
            }
            None => ProjectDirs::from("de", "conplement AG", "omnect-cli")
                .ok_or_else(|| anyhow::anyhow!("Application dirs not accessible"))?
                .data_dir()
                .to_path_buf(),
        };

        // check that key pair exists
        if let Some(key_path) = &priv_key_path {
            if !key_path.try_exists().is_ok_and(|exists| exists)
                || !key_path
                    .with_extension("pub")
                    .try_exists()
                    .is_ok_and(|exists| exists)
            {
                anyhow::bail!("Missing private/public ssh key.");
            }
        }

        Ok(Config {
            backend,
            dir: dir.clone(),
            priv_key_path,
            config_path: config_path.unwrap_or_else(|| dir.join(SSH_CONFIG_NAME)),
        })
    }

    pub fn set_backend(&mut self, backend: Url) {
        self.backend = backend;
    }
}

fn create_ssh_key_pair(priv_key_path: &Path, pub_key_path: &Path) -> Result<()> {
    // remove possibly existing key files first
    let _ = fs::remove_file(priv_key_path);
    let _ = fs::remove_file(pub_key_path);

    let output = Command::new("ssh-keygen")
        .stdin(Stdio::null()) // don't ask to overwrite existing key files
        .args(["-q"]) // silence
        .args(["-t", SSH_KEY_FORMAT])
        .args(["-N", ""]) // empty passphrase
        .args(["-f", priv_key_path.to_str().unwrap()]) // destination file
        .output()
        .map_err(|err| anyhow::anyhow!("Failed to create ssh key pair: {err}"))?;

    if !output.status.success() {
        let output = str::from_utf8(&output.stderr).unwrap();
        anyhow::bail!("Failed to create ssh key pair: {output}");
    }

    Ok(())
}

#[derive(Deserialize)]
struct SshTunnelInfo {
    #[serde(rename = "clientBastionCert")]
    bastion_cert: String,
    #[serde(rename = "clientDeviceCert")]
    device_cert: String,
    #[serde(rename = "host")]
    bastion_hostname: String,
    #[serde(rename = "port")]
    bastion_port: u16,
    #[serde(rename = "bastionUser")]
    bastion_username: String,
}

async fn request_ssh_tunnel(
    backend: &Url,
    device_id: &str,
    username: &str,
    ssh_pub_key: &str,
    access_token: AccessToken,
) -> Result<SshTunnelInfo> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PrepareTunnelArgs {
        device_id: String,
        key: String,
        user: String,
    }

    let prepare_tunnel_args = PrepareTunnelArgs {
        device_id: device_id.to_string(),
        key: ssh_pub_key.to_string(),
        user: username.to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post(backend.join(BACKEND_API_ENDPOINT)?)
        .json(&prepare_tunnel_args)
        .bearer_auth(access_token.secret())
        .send()
        .await
        .map_err(|err| anyhow::anyhow!("Failed to perform ssh tunnel request: {err}"))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Something went wrong while creating the ssh tunnel: {}",
            response.status().canonical_reason().unwrap()
        ); // safe
    }

    Ok(response.json().await?)
}

fn store_certs(
    cert_dir: &Path,
    bastion_cert: String,
    device_cert: String,
) -> Result<(PathBuf, PathBuf)> {
    let mut bastion_cert_path = cert_dir.join(BASTION_CERT_NAME);
    let mut device_cert_path = cert_dir.join(DEVICE_CERT_NAME);

    fs::write(&mut bastion_cert_path, bastion_cert)
        .map_err(|err| anyhow::anyhow!("Failed to store bastion certificate: {err}"))?;
    fs::write(&mut device_cert_path, device_cert)
        .map_err(|err| anyhow::anyhow!("Failed to store device certificate: {err}"))?;

    Ok((bastion_cert_path, device_cert_path))
}

struct BastionDetails {
    username: String,
    hostname: String,
    port: u16,
    priv_key: PathBuf,
    cert: PathBuf,
}

struct DeviceDetails {
    username: String,
    hostname: String,
    priv_key: PathBuf,
    cert: PathBuf,
}

fn create_ssh_config(
    config_path: &Path,
    bastion_details: BastionDetails,
    device_details: DeviceDetails,
) -> Result<()> {
    let config_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(config_path.to_str().unwrap())
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::AlreadyExists => {
                anyhow::anyhow!(
                    r#"ssh config file already exists and would be overwritten.
Please remove config file first."#
                )
            }
            _ => anyhow::anyhow!("Failed to create ssh config file: {err}"),
        })?;

    let mut writer = BufWriter::new(config_file);

    if let Ok("windows") = std::env::var("CONTAINER_HOST").as_deref() {
        writeln!(
            &mut writer,
            "\
Host bastion
	User {}
	Hostname {}
	Port {}
	IdentityFile ~/.ssh/{}
	CertificateFile ~/.ssh/{}
	ProxyCommand none

Host {}
	User {}
	IdentityFile ~/.ssh/{}
	CertificateFile ~/.ssh/{}
	ProxyCommand ssh bastion",
            bastion_details.username,
            bastion_details.hostname,
            bastion_details.port,
            bastion_details
                .priv_key
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(), // safe
            bastion_details.cert.file_name().unwrap().to_str().unwrap(), // safe
            device_details.hostname,
            device_details.username,
            device_details
                .priv_key
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(), // safe
            device_details.cert.file_name().unwrap().to_str().unwrap(), // safe
        )
        .map_err(|err| anyhow::anyhow!("Failed to write ssh config file: {err}"))?;
    } else {
        writeln!(
            &mut writer,
            "\
Host bastion
	User {}
	Hostname {}
	Port {}
	IdentityFile {}
	CertificateFile {}
	ProxyCommand none

Host {}
	User {}
	IdentityFile {}
	CertificateFile {}
	ProxyCommand ssh -F {} bastion",
            bastion_details.username,
            bastion_details.hostname,
            bastion_details.port,
            bastion_details.priv_key.to_str().unwrap(), // safe
            bastion_details.cert.to_str().unwrap(),     // safe
            device_details.hostname,
            device_details.username,
            device_details.priv_key.to_str().unwrap(), // safe
            device_details.cert.to_str().unwrap(),     // safe
            config_path.to_str().unwrap(),             // safe
        )
        .map_err(|err| anyhow::anyhow!("Failed to write ssh config file: {err}"))?;
    }

    Ok(())
}

fn print_ssh_tunnel_info(cert_dir: &Path, config_path: &Path, destination: &str) {
    println!("Successfully established ssh tunnel!");
    if let Ok("windows") = std::env::var("CONTAINER_HOST").as_deref() {
        println!(
            "You can ssh now to your device via its device name, e.g.:\nssh {}",
            destination
        );
    } else {
        println!("Certificate dir: {}", cert_dir.to_str().unwrap());
        println!("Configuration path: {}", config_path.to_str().unwrap());
        println!(
            "Use the configuration in \"{}\" to use the tunnel, e.g.:\nssh -F {} {}",
            config_path.to_str().unwrap(), // safe
            config_path.to_str().unwrap(), // safe
            destination
        );
    }
}

pub async fn ssh_create_tunnel(
    device: &str,
    username: &str,
    config: Config,
    access_token: oauth2::AccessToken,
) -> Result<()> {
    // setup place to store the certificates and configuration
    fs::create_dir_all(&config.dir)?;
    fs::create_dir_all(
        config
            .config_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid config path"))?,
    )?;

    // create ssh key pair, if necessary
    let (priv_key_path, pub_key_path) = match &config.priv_key_path {
        None => {
            let priv_key_path = config.dir.join(format!("id_{}", SSH_KEY_FORMAT));
            let pub_key_path = priv_key_path.with_extension("pub");

            create_ssh_key_pair(&priv_key_path, &pub_key_path)
                .map_err(|err| anyhow::anyhow!("Failed to create ssh key pair: {err}"))?;

            (priv_key_path, pub_key_path)
        }
        Some(key_path) => {
            if !key_path.try_exists().is_ok_and(|exists| exists)
                || !key_path
                    .with_extension("pub")
                    .try_exists()
                    .is_ok_and(|exists| exists)
            {
                anyhow::bail!("No such ssh key pair: \"{}\"", key_path.display());
            }
            (key_path.clone(), key_path.with_extension("pub"))
        }
    };

    let ssh_pub_key = fs::read_to_string(pub_key_path)
        .map_err(|err| anyhow::anyhow!("Failed to read public key: {err}"))?;

    let ssh_tunnel_info = request_ssh_tunnel(
        &config.backend,
        device,
        username,
        &ssh_pub_key,
        access_token,
    )
    .await?;

    let (bastion_cert, device_cert) = store_certs(
        &config.dir,
        ssh_tunnel_info.bastion_cert,
        ssh_tunnel_info.device_cert,
    )?;

    let bastion_details = BastionDetails {
        username: ssh_tunnel_info.bastion_username,
        hostname: ssh_tunnel_info.bastion_hostname,
        port: ssh_tunnel_info.bastion_port,
        priv_key: priv_key_path.clone(),
        cert: bastion_cert,
    };
    let device_details = DeviceDetails {
        username: username.to_string(),
        hostname: device.to_string(),
        priv_key: priv_key_path,
        cert: device_cert,
    };

    create_ssh_config(&config.config_path, bastion_details, device_details)?;

    print_ssh_tunnel_info(&config.dir, &config.config_path, device);

    Ok(())
}
