use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader,Error, ErrorKind, Read};
use std::path::{Path,PathBuf};

use bollard::auth::DockerCredentials;
use bollard::container::{Config, RemoveContainerOptions, LogOutput};
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;

use futures_executor::block_on;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;

//todo better via env var overwrite?
const DOCKER_REG: &'static str = "icsdm.azurecr.io";
const DOCKER_IMAGE: &'static str = "icsdm.azurecr.io/mlilien/ics-dm-cli-backend:latest";
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
        println!("wtf");
        return DockerCredentials{
            identitytoken: Some(identitytoken.to_string()),
            ..Default::default()
        }
    }
    else if "null" != auth
    {
        println!("auth: {}", auth);
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

async fn docker_exec(container_config: Config<&str>, exec_options: CreateExecOptions<&str>) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let docker = Docker::connect_with_unix_defaults().unwrap();

    match docker.image_history(DOCKER_IMAGE).await {
        Err(_e) => {
            //only pull the image if we don't have it locally available
            docker.create_image(Some(CreateImageOptions {from_image: DOCKER_IMAGE,
                                ..Default::default()}),
                                None,
                                Some(get_docker_cred())
                            ).try_collect::<Vec<_>>().await?;
        }
        Ok(_) => (())
    }

    let id = docker
        .create_container::<&str, &str>(None, container_config)
        .await?
        .id;
    docker.start_container::<String>(&id, None).await?;

    // non interactive
    let exec = docker
        .create_exec(
            &id,
            exec_options
        )
        .await?
        .id;

    let mut stream = docker.start_exec(&exec, None);
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

    docker
        .remove_container(
            &id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;
        Ok(())
}

#[tokio::main]
pub async fn inject_config(input_config_file: &str, target_config_file: &str, input_image_file: &str) -> Result<(),Error> {
    match block_on( async move {

        let host_config = HostConfig {
            // privileged for losetup in the container
            // @todo check how to restrict rights with capabilities instead
            privileged: Some(true),

            ..Default::default()
        };

        let mut volumes = HashMap::new();
        // to setup the image loop device properly we need to access the hosts devtmpfs
        volumes.insert("/dev/","/dev/");
        volumes.insert(input_image_file, TARGET_DEVICE_IMAGE);
        let input_config_file_path = Path::new(input_config_file);
        let input_config_file_docker = format!("/tmp/{:?}", input_config_file_path.file_name().unwrap().to_string_lossy());
        volumes.insert(input_config_file,input_config_file_docker.as_str());

        let container_config = Config {
            image: Some(DOCKER_IMAGE),
            tty: Some(true),
            host_config: Some(host_config),
            ..Default::default()
        };

        let exec_options = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["copy_file_to_image.sh", "-i", input_config_file, "-o", target_config_file]),
            //((cmd: Some(main_vec),
            ..Default::default()
        };
        docker_exec(container_config,exec_options).await?;
        Ok(())
    }) as Result<(), Box<dyn std::error::Error + 'static>>
    {
        Ok(_) => Ok(()),
        Err(e) => { eprintln!("{:#?})", e); Err(Error::from(ErrorKind::Other))}
    }
}

#[tokio::main]
pub async fn docker_version() -> Result<(), std::io::Error> {
    block_on( async move {
        let docker = Docker::connect_with_local_defaults().unwrap();
        let version = docker.version().await.unwrap();
        println!("docker version: {:#?}", version);
    });
    Ok(())
}
