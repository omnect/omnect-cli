use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};

use bollard::auth::DockerCredentials;
use bollard::container::{Config, LogOutput, LogsOptions};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::models::HostConfig;
use bollard::Docker;

use futures_executor::block_on;
use futures_util::TryStreamExt;

use super::validators::identity::{validate_identity, IdentityType};
use path_absolutize::Absolutize;
use tempfile::NamedTempFile;
use uuid::Uuid;

const DOCKER_IMAGE_NAME: &'static str = "ics-dm-cli-backend";
const DOCKER_IMAGE_VERSION: &'static str = env!("CARGO_PKG_VERSION");

lazy_static! {
    // read at compile time
    static ref DEFAULT_DOCKER_REG_NAME: &'static str = {
        if let Some(def_reg_name) = option_env!("ICS_DM_CLI_DOCKER_REG_NAME") {
            def_reg_name
        }
        else {
            "icsdm.azurecr.io"
        }
    };
    // read at run time
    static ref DOCKER_REG_NAME: String = {
        if let Some(reg_name) = env::var_os("ICS_DM_CLI_DOCKER_REG_NAME") {
            reg_name.into_string().unwrap()
        }
        else {
            (*DEFAULT_DOCKER_REG_NAME).to_string()
        }
    };
    static ref DOCKER_IMAGE_ID: String = format!("{}/{}:{}", *DOCKER_REG_NAME, DOCKER_IMAGE_NAME, DOCKER_IMAGE_VERSION);
}

fn get_docker_cred() -> Result<DockerCredentials, Box<dyn std::error::Error>> {
    let mut path = PathBuf::new();
    path.push(dirs::home_dir().unwrap());
    path.push(".docker/config.json");

    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader)?;
    let reg_name = DOCKER_REG_NAME.as_str();
    let identitytoken = json["auths"][reg_name]["identitytoken"]
        .to_string()
        .replace("\"", "");

    if "null" != identitytoken {
        return Ok(DockerCredentials {
            identitytoken: Some(identitytoken),
            ..Default::default()
        });
    }

    let auth = json["auths"][reg_name]["auth"]
        .to_string()
        .replace("\"", "");

    if "null" != auth {
        let byte_auth = base64::decode_config(auth, base64::STANDARD)?;
        let v: Vec<&str> = std::str::from_utf8(&byte_auth)?.split(":").collect();

        return Ok(DockerCredentials {
            username: Some(v[0].to_string()),
            password: Some(v[1].to_string()),
            ..Default::default()
        });
    }

    return Ok(DockerCredentials {
        ..Default::default()
    });
}

#[tokio::main]
async fn docker_exec(
    mut binds: Option<Vec<std::string::String>>,
    cmd: Option<Vec<&str>>,
) -> Result<(), Box<dyn std::error::Error>> {
    block_on(async move {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let mut filters = HashMap::new();
        let img_id = DOCKER_IMAGE_ID.as_str();

        info!("backend image id: {}", img_id);

        filters.insert("reference", vec![img_id]);

        let image_list = docker.list_images(Some(ListImagesOptions {
            all: true,
            filters,
            ..Default::default()
        }));

        if true == image_list.await?.is_empty() {
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

        debug!("Using binds: {:?}", binds);

        let host_config = HostConfig {
            auto_remove: Some(true),
            binds: binds,
            ..Default::default()
        };

        let mut env: Option<Vec<&str>> = None;
        if cfg!(debug_assertions) {
            let mut _env = Vec::new();
            _env.push("DEBUG=1");
            env = Some(_env);
        }

        let container_config = Config {
            image: Some(img_id),
            tty: Some(true),
            host_config: Some(host_config),
            env: env,
            cmd: cmd,
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
            Ok(Some(msg)) => docker_run_result = Err(Box::<dyn std::error::Error>::from(msg)),
            Err(e) => docker_run_result = Err(e),
            _ => {}
        };

        let contents = fs::read_to_string(path)?;

        if !contents.is_empty() {
            docker_run_result = Err(Box::<dyn std::error::Error>::from(contents))
        }
        match docker_run_result {
            Ok(()) => {
                debug!("Command ran successfully.");
            }
            _ => {} // Error will be printed by caller.
        }

        docker_run_result
    })
}

pub fn set_wifi_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
            cmd_exec(
                vec![config_file, image_file],
                |files| -> String {
                    format!("set_wifi_config.sh, -i, {0}, -w, {1}", files[0], files[1])
                },
                generate_bmap,
            )
        },
    )
}

pub fn set_enrollment_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    super::validators::enrollment::validate_enrollment(&config_file)?;

    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
            cmd_exec(
                vec![config_file, &image_file],
                |files| -> String {
                    format!(
                        "copy_file_to_image.sh, -i, {0}, -o, /etc/ics_dm/enrollment_static.json, -p, factory, -w, {1}",
                        files[0], files[1]
                    )
                },
                generate_bmap,
            )
        },
    )
}

pub fn set_iotedge_gateway_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    root_ca_file: &PathBuf,
    edge_device_identity_full_chain_file: &PathBuf,
    edge_device_identity_key_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_identity(IdentityType::Gateway, &config_file)?
        .iter()
        .for_each(|x| warn!("{}", x));

    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
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
                generate_bmap,
            )
        },
    )
}

pub fn set_iot_leaf_sas_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    root_ca_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_identity(IdentityType::Leaf, &config_file)?
        .iter()
        .for_each(|x| warn!("{}", x));

    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
            cmd_exec(
                vec![config_file, root_ca_file, image_file],
                |files| -> String {
                    format!(
                        "set_iot_leaf_config.sh, -c, {0}, -r, {1}, -w, {2}",
                        files[0], files[1], files[2]
                    )
                },
                generate_bmap,
            )
        },
    )
}

pub fn set_identity_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_identity(IdentityType::Standalone, &config_file)?
        .iter()
        .for_each(|x| warn!("{}", x));

    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
            cmd_exec(
                vec![config_file, image_file],
                |files| -> String {
                    format!(
                        "set_identity_config.sh, -c, {0}, -w, {1}",
                        files[0], files[1]
                    )
                },
                generate_bmap,
            )
        },
    )
}

pub fn set_iot_hub_device_update_config(
    config_file: &PathBuf,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
            cmd_exec(
                vec![config_file, image_file],
                |files| -> String {
                    format!("copy_file_to_image.sh, -i, {0}, -o, /etc/adu/adu-conf.txt, -p, factory, -w {1}",
                        files[0], files[1]
                    )
                },
                generate_bmap,
            )
        },
    )
}

pub fn set_boot_config(
    boot_script: &PathBuf,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    super::validators::image::validate_and_decompress_image(
        image_file,
        move |image_file: &PathBuf| -> Result<(), Box<(dyn std::error::Error)>> {
            cmd_exec(
                vec![boot_script, image_file],
                |files| -> String {
                    format!("copy_file_to_image.sh, -i, {0}, -o, /boot/boot.scr, -p, factory, -w {1}",
                        files[0], files[1]
                    )
                },
                generate_bmap,
            )
        },
    )
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

pub fn cmd_exec<F>(
    files: Vec<&PathBuf>,
    f: F,
    generate_bmap: bool,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&Vec<String>) -> String,
{
    let mut binds: Vec<String> = vec![];
    let mut bind_files: Vec<String> = vec![];
    let tmp_folder = Uuid::new_v4();

    // validate input files
    // create temporary bind paths
    files.iter().try_for_each(|&f| -> Result<(), Error> {
        let path = ensure_filepath(&f)?;
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
    let mut cmdline: Vec<String> = f(&bind_files).split(",").map(|s| s.to_string()).collect();

    if generate_bmap {
        let bmap_path = format!("{}.bmap", files.last().unwrap().to_str().unwrap());
        let bmap_pathbuf = PathBuf::from(&bmap_path);
        File::create(bmap_pathbuf.clone())?;
        let bmap_bind_path = format!(
            "/tmp/{}/{}",
            tmp_folder,
            bmap_pathbuf.file_name().unwrap().to_str().unwrap()
        );
        bind_files.push(bmap_bind_path.clone());
        binds.push(format!("{}:{}", bmap_path, bmap_bind_path));
        cmdline.push(String::from("-b"));
        cmdline.push(bind_files.last().unwrap().to_string());
    }

    docker_exec(
        Some(binds),
        Some(cmdline.iter().map(AsRef::as_ref).collect()),
    )
}

fn ensure_filepath(filepath: &PathBuf) -> Result<String, Error> {
    error_on_file_not_exists(&filepath)?;

    Ok(Path::new(filepath)
        .absolutize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string())
}

fn error_on_file_not_exists(file: &PathBuf) -> Result<(), Error> {
    std::fs::metadata(&file)
        .map_err(|e| Error::new(e.kind(), e.to_string() + ": " + file.to_str().unwrap()))?
        .is_file()
        .then(|| ())
        .ok_or(Error::new(
            ErrorKind::InvalidInput,
            file.to_str().unwrap().to_owned() + &" is not a file path",
        ))?;

    Ok(())
}
