use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};

extern crate path_absolutize;
use path_absolutize::*;

use bollard::auth::DockerCredentials;
use bollard::container::{Config, RemoveContainerOptions, LogOutput};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;

use futures_executor::block_on;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;

const DOCKER_REG: &'static str = "icsdm.azurecr.io";
const DOCKER_IMAGE: &'static str = "ics-dm-cli-backend";

const TARGET_DEVICE_IMAGE: &'static str = "/tmp/image.wic";

fn get_docker_cred() -> DockerCredentials {
    if cfg!(windows) {
        println!("Warning: Windows detected. We currently don't support the docker credential store.");
    }
    let mut path = PathBuf::new( );
    path.push(dirs::home_dir().unwrap());
    path.push(".docker/config.json");
    let file = File::open(&path).expect("Cannot open docker config.");
    let mut json_str = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut json_str).expect("Cannot read docker config.");
    let json : serde_json::Value  = serde_json::from_str(&json_str).expect("Cannot parse docker config.");
    let auth = &json["auths"][DOCKER_REG]["auth"].to_owned().to_string().replace("\"","");
    let identitytoken = &json["auths"][DOCKER_REG]["identitytoken"].to_owned().to_string().replace("\"","");

    if "null" != identitytoken
    {
        return DockerCredentials{
            identitytoken: Some(identitytoken.to_string()),
            ..Default::default()
        }
    }
    else if "null" != auth
    {
        let byte_auth = base64::decode_config(auth, base64::STANDARD).expect("Cannot base64 decode docker credentials");
        let dec_auth =  std::str::from_utf8(&byte_auth).expect("Cannot convert docker credentials.");
        let v : Vec<&str> = dec_auth.split(":").collect();
        return DockerCredentials{
            username: Some(v[0].to_owned().to_string()),
            password: Some(v[1].to_owned().to_string()),
            ..Default::default()
        }
    }
    else
    {
        return DockerCredentials{
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn docker_exec(container_config: Config<&str>, exec_options: CreateExecOptions<&str>) -> Result<(), Error> {
    match block_on( async move {
        let docker = Docker::connect_with_unix_defaults().unwrap();
        let image = format!("{}/{}:{}", DOCKER_REG,DOCKER_IMAGE, env!("CARGO_PKG_VERSION"));

        if let Err(_e) = docker.image_history(image.as_str()).await {
            docker.create_image(
                Some(CreateImageOptions {
                    from_image: image.as_str(),
                    ..Default::default()
                }),
                None,
                Some(get_docker_cred())
            ).try_collect::<Vec<_>>().await.unwrap();
        }

        let container = docker.create_container::<&str, &str>(None, container_config).await?;
        
        docker.start_container::<String>(&container.id, None).await?;

        // non interactive
        let exec = docker.create_exec(&container.id, exec_options).await?;

        let mut stream = docker.start_exec(&exec.id, None);

        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                StartExecResults::Attached{ log } => match log {
                    LogOutput::StdOut{ .. } => print!("{}", log),
                    LogOutput::StdErr{ .. } => eprint!("{}", log),
                    _ => {}
                }
                _ => {}
            }
        }

        docker.remove_container(&container.id, Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        ).await?;

        Ok(())

    }) as Result<(), Box<dyn std::error::Error + 'static>>
    {
        Ok(_) => Ok(()),
        Err(e) => { eprintln!("{:#?})", e); Err(Error::from(ErrorKind::Other))}
    }
}

pub fn set_wifi_config(config: &PathBuf, image: &PathBuf) -> Result<(), Error> {
    let input_config_file = ensure_filepath(&config)?;
    let input_image_file = ensure_filepath(&image)?;


    let mut binds :Vec<std::string::String> = Vec::new();
    // to setup the image loop device properly we need to access the hosts devtmpfs
    binds.push("/dev/:/dev/".to_owned().to_string());

    // input file binding
    binds.push(format!("{}:{}", input_image_file, TARGET_DEVICE_IMAGE));
    let target_input_config_file = format!("/tpm/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));

    let host_config = HostConfig {
        // privileged for losetup in the container
        // @todo check how to restrict rights with capabilities instead
        privileged: Some(true),
        binds: Some(binds),
        ..Default::default()
    };

    let image = format!("{}/{}:{}", DOCKER_REG,DOCKER_IMAGE, env!("CARGO_PKG_VERSION"));

    let container_config = Config {
        image: Some(image.as_str()),
        tty: Some(true),
        host_config: Some(host_config),
        ..Default::default()
    };

    // backend call
    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(vec!["set_wifi_config.sh", "-i", &target_input_config_file]),
        ..Default::default()
    };

    docker_exec(container_config, exec_options)
}

pub fn set_enrollment_config(enrollment_config_file: &PathBuf, provisioning_config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Error> {
    let input_enrollment_config_file = ensure_filepath(&enrollment_config_file)?;
    let input_provisioning_config_file = ensure_filepath(&provisioning_config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    
    let mut binds: Vec<std::string::String> = Vec::new();
    // to setup the image loop device properly we need to access the hosts devtmpfs
    binds.push("/dev/:/dev/".to_owned().to_string());

    // input file binding
    binds.push(format!("{}:{}", input_image_file, TARGET_DEVICE_IMAGE));
    let target_input_enrollment_config_file = format!("/tpm/{}", input_enrollment_config_file);
    binds.push(format!("{}:{}", input_enrollment_config_file, target_input_enrollment_config_file));
    let target_input_provisioning_config_file = format!("/tpm/{}", input_provisioning_config_file);
    binds.push(format!("{}:{}", input_provisioning_config_file, target_input_provisioning_config_file));

    let host_config = HostConfig {
        // privileged for losetup in the container
        // @todo check how to restrict rights with capabilities instead
        privileged: Some(true),
        binds: Some(binds),
        ..Default::default()
    };

    let image = format!("{}/{}:{}", DOCKER_REG,DOCKER_IMAGE, env!("CARGO_PKG_VERSION"));

    let container_config = Config {
        image: Some(image.as_str()),
        tty: Some(true),
        host_config: Some(host_config),
        ..Default::default()
    };

    // backend call
    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(vec!["set_enrollment_config.sh", "-e", &target_input_enrollment_config_file, "-p", &target_input_provisioning_config_file]),
        ..Default::default()
    };

    docker_exec(container_config, exec_options)
}

pub fn set_iotedge_gateway_config(config_file: &PathBuf, image_file: &PathBuf, root_ca_file: &PathBuf, edge_device_identity_full_chain_file: &PathBuf, edge_device_identity_key_file: &PathBuf) -> Result<(), Error> {
    let input_config_file = ensure_filepath(&config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let input_root_ca_file = ensure_filepath(&root_ca_file)?;
    let input_edge_device_identity_full_chain_file = ensure_filepath(&edge_device_identity_full_chain_file)?;
    let input_edge_device_identity_key_file = ensure_filepath(&edge_device_identity_key_file)?;
    
    
    let mut binds :Vec<std::string::String> = Vec::new();
    // to setup the image loop device properly we need to access the hosts devtmpfs
    binds.push("/dev/:/dev/".to_owned().to_string());

    // input file binding
    binds.push(format!("{}:{}", input_image_file, TARGET_DEVICE_IMAGE));
    let target_input_config_file = format!("/tpm/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));
    let target_input_root_ca_file = format!("/tpm/{}", input_root_ca_file);
    binds.push(format!("{}:{}", input_root_ca_file, target_input_root_ca_file));
    let target_input_edge_device_identity_full_chain_file = format!("/tpm/{}", input_edge_device_identity_full_chain_file);
    binds.push(format!("{}:{}", input_edge_device_identity_full_chain_file, target_input_edge_device_identity_full_chain_file));
    let target_input_edge_device_identity_key_file = format!("/tpm/{}", input_edge_device_identity_key_file);
    binds.push(format!("{}:{}", input_edge_device_identity_key_file, target_input_edge_device_identity_key_file));

    let host_config = HostConfig {
        // privileged for losetup in the container
        // @todo check how to restrict rights with capabilities instead
        privileged: Some(true),
        binds: Some(binds),
        ..Default::default()
    };

    let image = format!("{}/{}:{}", DOCKER_REG,DOCKER_IMAGE, env!("CARGO_PKG_VERSION"));

    let container_config = Config {
        image: Some(image.as_str()),
        tty: Some(true),
        host_config: Some(host_config),
        ..Default::default()
    };

    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(vec!["set_iotedge_gw_config.sh", "-i", &target_input_config_file, "-e", &target_input_edge_device_identity_full_chain_file, "-k", &target_input_edge_device_identity_key_file, "-r", &target_input_root_ca_file]),
        ..Default::default()
    };

    docker_exec(container_config, exec_options)
}

pub fn set_iotedge_sas_leaf_config(config_file: &PathBuf, image_file: &PathBuf, root_ca_file: &PathBuf) -> Result<(), Error> {
    let input_config_file = ensure_filepath(&config_file)?;
    let input_image_file = ensure_filepath(&image_file)?;
    let input_root_ca_file = ensure_filepath(&root_ca_file)?;    
    
    let mut binds :Vec<std::string::String> = Vec::new();
    // to setup the image loop device properly we need to access the hosts devtmpfs
    binds.push("/dev/:/dev/".to_owned().to_string());

    // input file binding
    binds.push(format!("{}:{}", input_image_file, TARGET_DEVICE_IMAGE));
    let target_input_config_file = format!("/tpm/{}", input_config_file);
    binds.push(format!("{}:{}", input_config_file, target_input_config_file));
    let target_input_root_ca_file = format!("/tpm/{}", input_root_ca_file);
    binds.push(format!("{}:{}", input_root_ca_file, target_input_root_ca_file));

    let host_config = HostConfig {
        // privileged for losetup in the container
        // @todo check how to restrict rights with capabilities instead
        privileged: Some(true),
        binds: Some(binds),
        ..Default::default()
    };

    let image = format!("{}/{}:{}", DOCKER_REG,DOCKER_IMAGE, env!("CARGO_PKG_VERSION"));

    let container_config = Config {
        image: Some(image.as_str()),
        tty: Some(true),
        host_config: Some(host_config),
        ..Default::default()
    };

    let exec_options = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(vec!["set_iotedge_leaf_sas_config.sh", "-i", &target_input_config_file, "-r", &target_input_root_ca_file]),
        ..Default::default()
    };

    docker_exec(container_config, exec_options)
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