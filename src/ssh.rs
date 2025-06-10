use std::convert::AsRef;
use std::fs;
use std::io::{BufWriter, prelude::*};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;

use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use oauth2::AccessToken;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use url::Url;

static BACKEND_API_ENDPOINT: &str = "/api/devices/prepareSSHConnection";
static SSH_KEY_FORMAT: &str = "ed25519";

static BASTION_CERT_NAME: &str = "bastion-cert.pub";
static DEVICE_CERT_NAME: &str = "device-cert.pub";

fn ssh_config_path(config_dir: &Path, device: &str) -> PathBuf {
    config_dir.join(format!("{device}_config"))
}

fn query_yes_no<R, W>(query: impl AsRef<str>, mut reader: R, mut writer: W) -> Result<bool>
where
    R: std::io::BufRead,
    W: std::io::Write,
{
    writeln!(writer, "{}", query.as_ref())?;

    loop {
        let mut buffer = String::new();
        reader
            .read_line(&mut buffer)
            .map(|err| anyhow::anyhow!("Can't read from stdin: {err}"))
            .context("create ssh configuration")?;

        match buffer.as_str() {
            "y" | "yes" => return Ok(true),
            "N" | "No" | "" => return Ok(false),
            _ => {
                eprintln!("Please specify either y(es) or N(o)");
            }
        }
    }
}

pub struct Config {
    backend: Url,
    dir: PathBuf,
    priv_key_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
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
            None => {
                let project_dirs = ProjectDirs::from("de", "conplement AG", "omnect-cli")
                    .ok_or_else(|| anyhow::anyhow!("Application dirs not accessible"))?;

                project_dirs
                    .runtime_dir()
                    .or_else(|| Some(project_dirs.config_dir()))
                    .unwrap()
                    .to_path_buf()
            }
        };

        // if user wants to use existing key pair, check that it exists
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

        // if user wants specific config file path, check whether an existing
        // config file would be overwritten. If so, query, whether this is
        // intended.
        if let Some(ref config_path) = config_path {
            if config_path.exists() {
                if query_yes_no(
                    format!(
                        r#"Config file "{}" would be overwritten by operation. Continue? [y/N]"#,
                        config_path.to_string_lossy(),
                    ),
                    std::io::BufReader::new(std::io::stdin()),
                    std::io::stderr(),
                )? {
                    log::info!(
                        "Overwriting existing config: {}",
                        config_path.to_string_lossy()
                    );
                } else {
                    anyhow::bail!("Not overwriting config.");
                }
            }
        }

        Ok(Config {
            backend,
            dir: dir.clone(),
            priv_key_path,
            config_path,
        })
    }

    pub fn config_path(&self, device: &str) -> PathBuf {
        match &self.config_path {
            Some(config_path) => config_path.clone(),
            None => ssh_config_path(&self.dir, device),
        }
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

async fn unpack_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| anyhow!("could not read response body: {err}"))?;

    if !status.is_success() {
        #[derive(Deserialize)]
        struct ErrorMessage {
            #[serde(rename = "internalMsg")]
            internal_message: String,
        }

        anyhow::bail!(
            serde_json::from_str::<ErrorMessage>(&body)
                .map(|err| err.internal_message)
                .unwrap_or_else(|_| "unknown error type".to_string())
        )
    } else {
        serde_json::from_str(&body).map_err(|_| anyhow!("unsupported reply."))
    }
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

    unpack_response(response)
        .await
        .map_err(|err| anyhow::anyhow!("Something went wrong creating the ssh tunnel: {err}"))
}

fn store_certs(
    device: &str,
    cert_dir: &Path,
    bastion_cert: String,
    device_cert: String,
) -> Result<(PathBuf, PathBuf)> {
    let mut bastion_cert_path = cert_dir.join(format!("{device}_{BASTION_CERT_NAME}"));
    let mut device_cert_path = cert_dir.join(format!("{device}_{DEVICE_CERT_NAME}"));

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
    log::info!(
        r#"creating new ssh config to: "{}""#,
        config_path.to_string_lossy()
    );

    let config_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(config_path.to_str().unwrap())
        .map_err(|err| {
            anyhow::anyhow!(
                r#"Failed to create ssh config file "{}": {}"#,
                config_path.to_string_lossy(),
                err.kind()
            )
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
    let device_config_path = config.config_path(device);

    // setup place to store the certificates and configuration
    fs::create_dir_all(&config.dir)?;
    fs::create_dir_all(
        device_config_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid config path"))?,
    )?;

    // create ssh key pair, if necessary
    let (priv_key_path, pub_key_path) = match &config.priv_key_path {
        None => {
            let priv_key_path = config.dir.join(format!("{}_id_{}", device, SSH_KEY_FORMAT));
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
        device,
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

    create_ssh_config(&device_config_path, bastion_details, device_details)?;

    print_ssh_tunnel_info(&config.dir, &device_config_path, device);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_query_yes_no_for_result(
        input: &str,
        expected_result: bool,
        expected_output: &str,
    ) -> bool {
        let input = std::io::Cursor::new(input);
        let mut output = vec![];

        let result = query_yes_no("Some test query", input, &mut output).unwrap();

        expected_result == result && expected_output != String::from_utf8(output).unwrap()
    }

    #[test]
    fn test_query_yes_no_true_on_y_input_succeess() {
        assert!(test_query_yes_no_for_result("y", true, ""));
    }

    #[test]
    fn test_query_yes_no_true_on_yes_input_succeess() {
        assert!(test_query_yes_no_for_result("yes", true, ""));
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_query_yes_no_true_on_N_input_succeess() {
        assert!(test_query_yes_no_for_result("N", false, ""));
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_query_yes_no_true_on_No_input_succeess() {
        assert!(test_query_yes_no_for_result("No", false, ""));
    }

    #[test]
    fn test_query_yes_no_true_on_default_input_succeess() {
        assert!(test_query_yes_no_for_result("", false, ""));
    }

    #[test]
    fn test_query_yes_no_multiple_input_succeess() {
        assert!(test_query_yes_no_for_result(
            "123\n345\nyes",
            true,
            "Please specify either y(es) or N(o)\nPlease specify either y(es) or N(o)"
        ));
    }

    #[test]
    fn test_query_yes_no_false_on_missing_correct_input_succeess() {
        assert!(test_query_yes_no_for_result(
            "123\n345",
            false,
            "Please specify either y(es) or N(o)\nPlease specify either y(es) or N(o)"
        ));
    }
}
