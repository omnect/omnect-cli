use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use bollard::Docker;
use bollard::auth::DockerCredentials;
use bollard::container::{Config, LogOutput, LogsOptions};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::models::HostConfig;

use futures_executor::block_on;
use futures_util::TryStreamExt;

use path_absolutize::Absolutize;
use uuid::Uuid;
use tempfile::NamedTempFile;

const DOCKER_REG_NAME: &'static str = include!("../gen/ICS_DM_CLI_DOCKER_REG_NAME.txt");
const DOCKER_IMAGE_NAME: &'static str = "ics-dm-cli-backend";
const DOCKER_IMAGE_VERSION: &'static str = env!("CARGO_PKG_VERSION");
const TARGET_DEVICE_IMAGE: &'static str = "image.wic";

lazy_static! {
    static ref DOCKER_IMAGE_ID: String = format!("{}/{}:{}", DOCKER_REG_NAME, DOCKER_IMAGE_NAME, DOCKER_IMAGE_VERSION);
}

fn get_docker_cred() -> Result<DockerCredentials, Box<dyn std::error::Error>> {
    let mut path = PathBuf::new();
    path.push(dirs::home_dir().unwrap());
    path.push(".docker/config.json");

    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader)?;
    let identitytoken = json["auths"][DOCKER_REG_NAME]["identitytoken"].to_string().replace("\"","");

    if "null" != identitytoken {
        return Ok(DockerCredentials {
            identitytoken: Some(identitytoken),
            ..Default::default()
        })
    }

    let auth = json["auths"][DOCKER_REG_NAME]["auth"].to_string().replace("\"","");

    if "null" != auth {
        let byte_auth = base64::decode_config(auth, base64::STANDARD)?;
        let v : Vec<&str> =  std::str::from_utf8(&byte_auth)?.split(":").collect();

        return Ok(DockerCredentials {
            username: Some(v[0].to_string()),
            password: Some(v[1].to_string()),
            ..Default::default()
        })
    }

    return Ok(DockerCredentials{
        ..Default::default()
    })
}

#[tokio::main]
async fn docker_exec(mut binds: Option<Vec<std::string::String>>, cmd: Option<Vec<&str>>) -> Result<(), Box<dyn std::error::Error>> {
    block_on( async move {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let mut filters = HashMap::new();
        filters.insert("reference", vec![DOCKER_IMAGE_ID.as_str()]);

        let image_list = docker.list_images(
            Some(ListImagesOptions {
                all: true,
                filters,
                ..Default::default()
            })
        );

        if true == image_list.await?.is_empty() {
            docker.create_image(
                Some(CreateImageOptions {
                    from_image: DOCKER_IMAGE_ID.as_str(),
                    ..Default::default()
                }),
                None,
                Some(get_docker_cred()?)
            ).try_collect::<Vec<_>>().await?;
        }

        let file = NamedTempFile::new()?;
        let target_error_file_path = format!("/tmp/{}", file.as_ref().file_name().unwrap().to_str().unwrap());

        if let Some(mut vec) = binds {
            vec.push(format!("{}:{}", file.as_ref().to_str().unwrap(), target_error_file_path));
            binds = Some(vec);
        }

        let host_config = HostConfig {
            auto_remove: Some(true),
            // we need cap_add + device_cgroup_rules to enable losetup inside the container
            cap_add: Some(vec!["SYS_ADMIN".to_string()]),
            // first rule: allow mknod,read/write of /dev/loopX, second rule: allow read/write of /dev/loopXpY
            device_cgroup_rules: Some(vec!["b 7:* rmw".to_string(), "b 259:* rw".to_string()]),
            binds: binds,
            ..Default::default()
        };

        let mut env : Option<Vec<&str>> = None;
        if cfg!(debug_assertions) {
            let mut _env = Vec::new();
            _env.push("DEBUG=1");
            env = Some(_env);
        }

        let container_config = Config {
            image: Some(DOCKER_IMAGE_ID.as_str()),
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

        let container = docker.create_container::<&str, &str>(None, container_config).await?;

        // by this block we ensure that docker.remove_container container is called
        // even if an error occured before
        let run_container_result = async {
            docker.start_container::<String>(&container.id, None).await?;

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
                match log {
                    LogOutput::StdIn{ .. } => {
                        print!("stdin: {}", log)
                    },
                    LogOutput::StdOut{ .. } => {
                        print!("stdout: {}", log)
                    },
                    LogOutput::Console{ .. } => {
                        print!("console: {}", log)
                    },
                    LogOutput::StdErr{ .. } => {
                        eprintln!("stderr: {}", log);
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

        docker_run_result
    })
}

pub fn set_wifi_config(config: &PathBuf, image: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let input_config_file = ensure_filepath(&config)?;
    let input_image_file = ensure_filepath(&image)?;
    let mut binds: Vec<std::string::String> = Vec::new();

    // input file binding
    let target_input_image_file = format!("/tmp/{}/{}", Uuid::new_v4(), TARGET_DEVICE_IMAGE);
    binds.push(format!("{}:{}", input_image_file, target_input_image_file));
    let target_input_config_file = format!("/tmp/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));

    docker_exec(Some(binds), Some(vec!["set_wifi_config.sh", "-i", &target_input_config_file, "-w", target_input_image_file.as_str()]))
}

pub fn set_enrollment_config(enrollment_config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let input_enrollment_config_file = ensure_filepath(&enrollment_config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let mut binds: Vec<std::string::String> = Vec::new();

    // input file binding
    let target_input_image_file = format!("/tmp/{}/{}", Uuid::new_v4(), TARGET_DEVICE_IMAGE);
    binds.push(format!("{}:{}", input_image_file, target_input_image_file));
    let target_input_enrollment_config_file = format!("/tmp/{}", input_enrollment_config_file);
    binds.push(format!("{}:{}", input_enrollment_config_file, target_input_enrollment_config_file));

    let mut command_vec = vec!["create_dir.sh", "-d", "upper/ics_dm", "-p", "etc", "-w", target_input_image_file.as_str(), "-g", "enrollment", "-m", "0775"];
    let mut command2_vec = vec!["copy_file_to_image.sh", "-i", &target_input_enrollment_config_file, "-o", "upper/ics_dm/enrollment_static.conf", "-p", "etc", "-w", target_input_image_file.as_str(), "-g", "enrollment", "-m", "0664"];
    command_vec.push("&&");
    command_vec.append(&mut command2_vec);
    docker_exec(Some(binds), Some(command_vec))
}

pub fn set_iotedge_gateway_config(config_file: &PathBuf, image_file: &PathBuf, root_ca_file: &PathBuf, edge_device_identity_full_chain_file: &PathBuf, edge_device_identity_key_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let input_config_file = ensure_filepath(&config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let input_root_ca_file = ensure_filepath(&root_ca_file)?;
    let input_edge_device_identity_full_chain_file = ensure_filepath(&edge_device_identity_full_chain_file)?;
    let input_edge_device_identity_key_file = ensure_filepath(&edge_device_identity_key_file)?;
    let mut binds :Vec<std::string::String> = Vec::new();

    // input file binding
    let target_input_image_file = format!("/tmp/{}/{}", Uuid::new_v4(), TARGET_DEVICE_IMAGE);
    binds.push(format!("{}:{}", input_image_file, target_input_image_file));
    let target_input_config_file = format!("/tmp/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));
    let target_input_root_ca_file = format!("/tmp/{}", input_root_ca_file);
    binds.push(format!("{}:{}", input_root_ca_file, target_input_root_ca_file));
    let target_input_edge_device_identity_full_chain_file = format!("/tmp/{}", input_edge_device_identity_full_chain_file);
    binds.push(format!("{}:{}", input_edge_device_identity_full_chain_file, target_input_edge_device_identity_full_chain_file));
    let target_input_edge_device_identity_key_file = format!("/tmp/{}", input_edge_device_identity_key_file);
    binds.push(format!("{}:{}", input_edge_device_identity_key_file, target_input_edge_device_identity_key_file));

    docker_exec(Some(binds), Some(vec!["set_iotedge_gw_config.sh", "-c", &target_input_config_file, "-e", &target_input_edge_device_identity_full_chain_file, "-k", &target_input_edge_device_identity_key_file, "-r", &target_input_root_ca_file, "-w", target_input_image_file.as_str()]))
}

pub fn set_iot_leaf_sas_config(config_file: &PathBuf, image_file: &PathBuf, root_ca_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let input_config_file = ensure_filepath(&config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let input_root_ca_file = ensure_filepath(&root_ca_file)?;

    let mut binds :Vec<std::string::String> = Vec::new();

    // input file binding
    let target_input_image_file = format!("/tmp/{}/{}", Uuid::new_v4(), TARGET_DEVICE_IMAGE);
    binds.push(format!("{}:{}", input_image_file, target_input_image_file));
    let target_input_config_file = format!("/tmp/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));
    let target_input_root_ca_file = format!("/tmp/{}", input_root_ca_file);
    binds.push(format!("{}:{}", input_root_ca_file, target_input_root_ca_file));

    docker_exec(Some(binds), Some(vec!["set_iot_leaf_config.sh", "-c", &target_input_config_file, "-r", &target_input_root_ca_file, "-w", target_input_image_file.as_str()]))
}

pub fn set_identity_config(config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let input_config_file = ensure_filepath(&config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let mut binds :Vec<std::string::String> = Vec::new();

    // input file binding
    let target_input_image_file = format!("/tmp/{}/{}", Uuid::new_v4(), TARGET_DEVICE_IMAGE);
    binds.push(format!("{}:{}", input_image_file, target_input_image_file));
    let target_input_config_file = format!("/tmp/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));

    docker_exec(Some(binds), Some(vec!["set_identity_config.sh", "-c", &target_input_config_file, "-w", target_input_image_file.as_str()]))
}

pub fn set_iot_hub_device_update_config(iot_hub_device_update_config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let input_iot_hub_device_update_config_file = ensure_filepath(&iot_hub_device_update_config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let mut binds: Vec<std::string::String> = Vec::new();

    // input file binding
    let target_input_image_file = format!("/tmp/{}/{}", Uuid::new_v4(), TARGET_DEVICE_IMAGE);
    binds.push(format!("{}:{}", input_image_file, target_input_image_file));
    let target_input_iot_hub_device_update_config_file = format!("/tmp/{}", input_iot_hub_device_update_config_file);
    binds.push(format!("{}:{}", input_iot_hub_device_update_config_file, target_input_iot_hub_device_update_config_file));

    docker_exec(Some(binds), Some(vec!["copy_file_to_image.sh", "-i", &target_input_iot_hub_device_update_config_file, "-o", "upper/adu/adu-conf.txt", "-p", "etc", "-w", target_input_image_file.as_str(), "-g", "adu", "-u", "adu", "-m", "0664"]))
}

#[tokio::main]
pub async fn docker_version() -> Result<(), Error> {
    block_on( async move {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let version = docker.version().await.unwrap();
        println!("docker version: {:#?}", version);
    });
    Ok(())
}

fn ensure_filepath(filepath: &PathBuf) -> Result<String, Error> {
    error_on_file_not_exists(&filepath)?;

    Ok(Path::new(filepath).absolutize().unwrap().to_str().unwrap().to_string())
}

fn error_on_file_not_exists(file: &PathBuf) -> Result<(), Error> {
    std::fs::metadata(&file)
    .map_err(|e| {Error::new(e.kind(), e.to_string() + ": " + file.to_str().unwrap())})?
    .is_file()
    .then(|| ())
    .ok_or(Error::new(ErrorKind::InvalidInput, file.to_str().unwrap().to_owned() + &" is not a file path"))?;

    Ok(())
}
