use super::cli::Partition;
use super::validators::identity::{validate_identity, IdentityType};
use super::validators::ssh::validate_ssh_pub_key;
use anyhow::{anyhow, Context, Result};
use bollard::auth::DockerCredentials;
use bollard::container::{Config, LogOutput, LogsOptions};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::models::HostConfig;
use bollard::Docker;
use futures_executor::block_on;
use futures_util::TryStreamExt;
use log::{debug, error, info, warn};
use path_absolutize::Absolutize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use uuid::Uuid;

const DOCKER_IMAGE_NAME: &str = "omnect-cli-backend";
const DOCKER_IMAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_export]
macro_rules! img_to_bmap_path {
    ($generate_bmap:expr, $wic_image_path:expr) => {{
        $generate_bmap.then_some({
            let mut bmap_image_path = $wic_image_path.clone();

            let res_image_path = loop {
                match bmap_image_path.extension() {
                    Some(e) if "wic" == e => break bmap_image_path.with_extension("wic.bmap"),
                    None => break bmap_image_path.with_extension("bmap"),
                    _ => {
                        dbg!(&bmap_image_path);
                        bmap_image_path = bmap_image_path.with_extension("");
                    }
                }
            };

            res_image_path
        })
    }};
}

lazy_static! {
    // read at compile time
    static ref DEFAULT_DOCKER_REG_NAME: &'static str = {
        if let Some(def_reg_name) = option_env!("OMNECT_CLI_DOCKER_REG_NAME") {
            def_reg_name
        }
        else {
            "omnectweucopsacr.azurecr.io"
        }
    };
    // read at run time
    static ref DOCKER_REG_NAME: String = {
        if let Some(reg_name) = env::var_os("OMNECT_CLI_DOCKER_REG_NAME") {
            reg_name.into_string().unwrap()
        }
        else {
            (*DEFAULT_DOCKER_REG_NAME).to_string()
        }
    };
    static ref DOCKER_IMAGE_ID: String = format!("{}/{}:{}", *DOCKER_REG_NAME, DOCKER_IMAGE_NAME, DOCKER_IMAGE_VERSION);
}

fn get_docker_cred() -> Result<DockerCredentials> {
    let mut path = PathBuf::new();
    path.push(dirs::home_dir().unwrap());
    path.push(".docker/config.json");

    let file = File::open(&path).context("get_docker_cred: open docker/config.json")?;
    let reader = BufReader::new(file);
    let json: serde_json::Value =
        serde_json::from_reader(reader).context("get_docker_cred: read docker/config.json")?;
    let reg_name = DOCKER_REG_NAME.as_str();
    let identitytoken = json["auths"][reg_name]["identitytoken"]
        .to_string()
        .replace('\"', "");

    if "null" != identitytoken {
        return Ok(DockerCredentials {
            identitytoken: Some(identitytoken),
            ..Default::default()
        });
    }

    let auth = json["auths"][reg_name]["auth"]
        .to_string()
        .replace('\"', "");

    if "null" != auth {
        let byte_auth = base64::decode_config(auth, base64::STANDARD)
            .context("get_docker_cred: decode auth")?;
        let v: Vec<&str> = std::str::from_utf8(&byte_auth)
            .context("get_docker_cred: split auth")?
            .split(':')
            .collect();

        return Ok(DockerCredentials {
            username: Some(v[0].to_string()),
            password: Some(v[1].to_string()),
            ..Default::default()
        });
    }

    Ok(DockerCredentials {
        ..Default::default()
    })
}

#[tokio::main]
async fn docker_exec(mut binds: Option<Vec<std::string::String>>, cmd: Vec<&str>) -> Result<()> {
    block_on(async move {
        let docker = Docker::connect_with_local_defaults()
            .context("docker_exec: connect to local docker")?;
        let mut filters = HashMap::new();
        let img_id = DOCKER_IMAGE_ID.as_str();

        anyhow::ensure!(!cmd.is_empty(), "docker_exec: empty command was passed");

        info!("docker_exec: try running {cmd:?} on image: {img_id}");

        filters.insert("reference", vec![img_id]);

        let image_list = docker.list_images(Some(ListImagesOptions {
            all: true,
            filters,
            ..Default::default()
        }));

        if image_list.await?.is_empty() {
            debug!("Image not already present, creating it.");
            docker
                .create_image(
                    Some(CreateImageOptions {
                        from_image: img_id,
                        ..Default::default()
                    }),
                    None,
                    Some(get_docker_cred()?),
                )
                .try_collect::<Vec<_>>()
                .await?;
            debug!("Image created.");
        }

        let file = NamedTempFile::new()?;
        let target_error_file_path = format!(
            "/tmp/{}",
            file.as_ref().file_name().unwrap().to_str().unwrap()
        );

        if let Some(mut vec) = binds {
            vec.push(format!(
                "{}:{}",
                file.as_ref().to_str().unwrap(),
                target_error_file_path
            ));
            binds = Some(vec);
        }

        info!("Using binds: {:?}", binds);

        let host_config = HostConfig {
            auto_remove: Some(true),
            binds,
            ..Default::default()
        };

        let mut env: Option<Vec<&str>> = None;
        if cfg!(debug_assertions) {
            env = Some(vec!["DEBUG=1"]);
        }

        let container_config = Config {
            image: Some(img_id),
            tty: Some(true),
            host_config: Some(host_config),
            env,
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            entrypoint: Some(vec!["entrypoint.sh", &target_error_file_path]),
            ..Default::default()
        };

        // close temp file, but keep the path to it around
        let path = file.into_temp_path();

        debug!("Creating container instance.");
        let container = docker
            .create_container::<&str, &str>(None, container_config)
            .await?;
        debug!("Created container instance.");

        // by this block we ensure that docker.remove_container container is called
        // even if an error occured before
        let run_container_result = async {
            debug!("Starting container instance.");
            docker
                .start_container::<String>(&container.id, None)
                .await?;
            debug!("Started container instance.");

            let logs_options = Some(LogsOptions {
                follow: true,
                stdout: true,
                stderr: true,
                tail: "all",
                ..Default::default()
            });

            let mut stream_error_log: Option<String> = None;
            let mut stream = docker.logs(&container.id, logs_options);

            while let Some(log) = stream.try_next().await? {
                let mut line_without_nl = format!("{}", log);
                if line_without_nl.ends_with('\n') {
                    line_without_nl.pop();
                }
                if line_without_nl.ends_with('\r') {
                    line_without_nl.pop();
                }
                match log {
                    LogOutput::StdIn { .. } => {
                        info!("stdin: {}", line_without_nl)
                    }
                    LogOutput::StdOut { .. } => {
                        info!("stdout: {}", line_without_nl)
                    }
                    LogOutput::Console { .. } => {
                        info!("console: {}", line_without_nl)
                    }
                    LogOutput::StdErr { .. } => {
                        error!("stderr: {}", line_without_nl);
                        // save error string to
                        stream_error_log = Some(log.to_string());
                        break;
                    }
                }
            }
            Ok(stream_error_log)
        };

        let mut docker_run_result = Ok(());

        match run_container_result.await {
            // if result has error string convert it to error
            Ok(Some(msg)) => docker_run_result = Err(anyhow!(msg)),
            Err(e) => docker_run_result = Err(e),
            _ => {}
        };

        let contents = fs::read_to_string(path)?;

        if !contents.is_empty() {
            docker_run_result = Err(anyhow!(contents))
        }
        if let Ok(()) = docker_run_result {
            debug!("Command ran successfully.");
        }

        docker_run_result
    })
}

pub fn set_iotedge_gateway_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    root_ca_file: &PathBuf,
    edge_device_identity_full_chain_file: &PathBuf,
    edge_device_identity_key_file: &PathBuf,
    bmap_file: Option<PathBuf>,
) -> Result<()> {
    validate_identity(IdentityType::Gateway, config_file, &None)?
        .iter()
        .for_each(|x| warn!("{}", x));

    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            cmd_exec(
                vec![
                    config_file,
                    edge_device_identity_full_chain_file,
                    edge_device_identity_key_file,
                    root_ca_file,
                    image_file,
                ],
                |files| -> String {
                    format!(
                        "set_iotedge_gw_config.sh, -c, {0}, -e, {1}, -k, {2}, -r, {3}, -w, {4}",
                        files[0], files[1], files[2], files[3], files[4]
                    )
                },
                bmap_file,
            )
        },
    )
}

pub fn set_iot_leaf_sas_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    root_ca_file: &PathBuf,
    bmap_file: Option<PathBuf>,
) -> Result<()> {
    validate_identity(IdentityType::Leaf, config_file, &None)?
        .iter()
        .for_each(|x| warn!("{}", x));

    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            cmd_exec(
                vec![config_file, root_ca_file, image_file],
                |files| -> String {
                    format!(
                        "set_iot_leaf_config.sh, -c, {0}, -r, {1}, -w, {2}",
                        files[0], files[1], files[2]
                    )
                },
                bmap_file,
            )
        },
    )
}

pub fn set_ssh_tunnel_certificate(
    image_file: &PathBuf,
    root_ca_file: &PathBuf,
    device_principal: &str,
    bmap_file: Option<PathBuf>,
) -> Result<()> {
    validate_ssh_pub_key(root_ca_file)?;

    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            cmd_exec(
                vec![root_ca_file, image_file],
                |files| -> String {
                    format!(
                        "set_ssh_tunnel_cert.sh, -r, {0}, -d, {1}, -w, {2}",
                        files[0], device_principal, files[1]
                    )
                },
                bmap_file,
            )
        },
    )
}

pub fn set_identity_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    bmap_file: Option<PathBuf>,
    payload: Option<PathBuf>,
) -> Result<()> {
    validate_identity(IdentityType::Standalone, config_file, &payload)?
        .iter()
        .for_each(|x| warn!("{}", x));

    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            if let Some(payload) = payload {
                cmd_exec(
                    vec![&payload, image_file],
                    |files| -> String {
                        format!(
                            "copy_file_to_image.sh, -i, {0}, -o, /etc/omnect/dsp-payload.json, -p, factory, -w {1}",
                            files[0], files[1]
                        )
                    },
                    None,
                )?;
            }

            cmd_exec(
                vec![config_file, image_file],
                |files| -> String {
                    format!(
                        "set_identity_config.sh, -c, {0}, -w, {1}",
                        files[0], files[1]
                    )
                },
                bmap_file,
            )
        },
    )
}

pub fn set_device_cert(
    intermediate_full_chain_cert_path: &PathBuf,
    device_full_chain_cert: &Vec<u8>,
    device_key: &Vec<u8>,
    image_file: &PathBuf,
    bmap_file: Option<PathBuf>,
) -> Result<()> {
    let uuid = Uuid::new_v4();
    let device_cert_path = &PathBuf::from(format!("/tmp/{}.pem", uuid));
    let device_key_path = &PathBuf::from(format!("/tmp/{}.key.pem", uuid));

    fs::write(device_cert_path, device_full_chain_cert)
        .context("set_device_cert: write device_cert_path")?;
    fs::write(device_key_path, device_key).context("set_device_cert: write device_key_path")?;

    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            cmd_exec(
                vec![device_cert_path, image_file],
                |files| -> String {
                    format!(
                        "copy_file_to_image.sh, -i, {0}, -o, /priv/device_id_cert.pem, -p, cert, -w {1}",
                        files[0], files[1]
                    )
                },
                None,
            )?;
            cmd_exec(
                vec![device_key_path, image_file],
                |files| -> String {
                    format!("copy_file_to_image.sh, -i, {0}, -o, /priv/device_id_cert_key.pem, -p, cert, -w {1}",
                        files[0], files[1]
                    )
                },
                None,
            )?;
            cmd_exec(
                vec![intermediate_full_chain_cert_path, image_file],
                |files| -> String {
                    format!(
                        "copy_file_to_image.sh, -i, {0}, -o, /priv/ca.crt.pem, -p, cert, -w {1}",
                        files[0], files[1]
                    )
                },
                None,
            )?;
            // copy as crt file for device cert store -> device can talk to our own services besides est
            cmd_exec(
                vec![intermediate_full_chain_cert_path, image_file],
                |files| -> String {
                    format!(
                        "copy_file_to_image.sh, -i, {0}, -o, /ca/ca.crt, -p, cert, -w {1}",
                        files[0], files[1]
                    )
                },
                bmap_file,
            )
        },
    )
}

pub fn set_iot_hub_device_update_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    bmap_file: Option<PathBuf>,
) -> Result<()> {
    let file =
        File::open(config_file).context("set_iot_hub_device_update_config: open config_file")?;
    serde_json::from_reader::<_, serde_json::Value>(BufReader::new(file))
        .context("set_iot_hub_device_update_config: read config_file")?;

    copy_to_image(
        config_file,
        image_file,
        Partition::factory,
        "/etc/adu/du-config.json".to_string(),
        bmap_file,
    )
}

pub fn copy_to_image(
    file: &PathBuf,
    image_file: &PathBuf,
    partition: Partition,
    destination: String,
    bmap_file: Option<PathBuf>,
) -> Result<()> {
    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            cmd_exec(
                vec![file, image_file],
                |files| -> String {
                    format!(
                    "copy_file_to_image.sh, -i, {0}, -o, {destination}, -p, {partition:?}, -w {1}",
                    files[0], files[1],
                )
                },
                bmap_file,
            )
        },
    )
}

pub fn copy_from_image(
    file: String,
    image_file: &PathBuf,
    partition: Partition,
    destination: &PathBuf,
) -> Result<()> {
    let tmp_file = PathBuf::from(format!("{}", Uuid::new_v4()));
    let tmp_file_clone = tmp_file.clone();
    File::create(&tmp_file).context("copy_from_image: create temporary destination")?;

    let result = super::validators::image::image_action(
        image_file,
        false,
        move |image_file: &PathBuf| -> Result<()> {
            cmd_exec(
                vec![&tmp_file, image_file],
                |files| -> String {
                    format!(
                        "copy_file_from_image.sh, -i, {file}, -o, {0}, -p, {partition:?}, -w {1}",
                        files[0], files[1],
                    )
                },
                None,
            )
        },
    );

    if result.is_ok() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).context("copy_from_image: create destination folder")?;
        }
        fs::rename(tmp_file_clone.clone(), destination)
            .map_err(anyhow::Error::from)
            .context("copy_from_image: move temporary file to destination")?;
    }

    if Path::new(&tmp_file_clone).exists() {
        fs::remove_file(tmp_file_clone).context("copy_from_image: remove temporary")?;
    }

    result
}

#[tokio::main]
pub async fn docker_version() -> Result<(), Error> {
    block_on(async move {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let version = docker.version().await.unwrap();
        info!("docker version: {:#?}", version);
    });
    Ok(())
}

pub fn cmd_exec<F>(files: Vec<&PathBuf>, f: F, bmap_file: Option<PathBuf>) -> Result<()>
where
    F: Fn(&Vec<String>) -> String,
{
    let mut binds: Vec<String> = vec![];
    let mut bind_files: Vec<String> = vec![];
    let tmp_folder = Uuid::new_v4();

    // validate input files
    // create temporary bind paths
    files.iter().try_for_each(|&f| -> Result<(), Error> {
        let path = ensure_filepath(f)?;
        let bind_path = format!(
            "/tmp/{}/{}",
            tmp_folder,
            f.file_name().unwrap().to_str().unwrap()
        );
        bind_files.push(bind_path.clone());
        binds.push(format!("{}:{}", path, bind_path));
        Ok(())
    })?;

    // format cmdline
    let mut cmdline: Vec<String> = f(&bind_files).split(',').map(|s| s.to_string()).collect();

    if bmap_file.is_some() {
        let bmap_file = &bmap_file
            .unwrap()
            .absolutize()
            .context("cmd_exec: get bmap file path")?
            .to_path_buf();
        File::create(bmap_file).context("cmd_exec: create bmap file")?;
        let bmap_bind_path = format!(
            "/tmp/{}/{}",
            tmp_folder,
            bmap_file.file_name().unwrap().to_str().unwrap()
        );
        bind_files.push(bmap_bind_path.clone());
        binds.push(format!(
            "{}:{}",
            bmap_file.to_string_lossy(),
            bmap_bind_path
        ));
        cmdline.push(String::from("-b"));
        cmdline.push(bind_files.last().unwrap().to_string());
    }

    docker_exec(Some(binds), cmdline.iter().map(AsRef::as_ref).collect())
}

fn ensure_filepath(filepath: &PathBuf) -> Result<String, Error> {
    error_on_file_not_exists(filepath)?;

    Ok(Path::new(filepath)
        .absolutize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string())
}

fn error_on_file_not_exists(file: &PathBuf) -> Result<(), Error> {
    std::fs::metadata(file)
        .map_err(|e| Error::new(e.kind(), e.to_string() + ": " + file.to_str().unwrap()))?
        .is_file()
        .then_some(())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                file.to_str().unwrap().to_owned() + " is not a file path",
            )
        })?;

    Ok(())
}
