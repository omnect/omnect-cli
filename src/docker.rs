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
    let identitytoken = json["auths"][reg_name]["identitytoken"].to_string().replace("\"","");

    if "null" != identitytoken {
        return Ok(DockerCredentials {
            identitytoken: Some(identitytoken),
            ..Default::default()
        })
    }

    let auth = json["auths"][reg_name]["auth"].to_string().replace("\"","");

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
        let img_id = DOCKER_IMAGE_ID.as_str();

        println!("backend image id: {}", img_id);

        filters.insert("reference", vec![img_id]);

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
                    from_image: img_id,
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
            // first cgroup rule: allow mknod,read/write of /dev/loop-control
            // second cgroup rule:  allow mknod,read/write of /dev/loopX
            // third cgroup rule: allow read/write of /dev/loopXpY
            device_cgroup_rules: Some(vec!["c 10:237 rmw".to_string(), "b 7:* rmw".to_string(), "b 259:* rw".to_string()]),
            security_opt: Some(vec!["seccomp=unconfined".to_string(),"apparmor=unconfined".to_string()]),
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

pub fn set_wifi_config(config_file: &PathBuf, image: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (binds, files) = prepare_binds(vec![config_file, image])?;

    docker_exec(Some(binds), Some(vec!["set_wifi_config.sh", "-i", &files[0], "-w", &files[1]]))
}

pub fn set_enrollment_config(config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (binds, files) = prepare_binds(vec![config_file, image_file])?;

    docker_exec(Some(binds), Some(vec![
        "create_dir.sh", "-d", "upper/ics_dm", "-p", "etc", "-w", &files[1], "-g", "enrollment", "-m", "0775",
        "&&",
        "copy_file_to_image.sh", "-i", &files[0], "-o", "upper/ics_dm/enrollment_static.conf", "-p", "etc", "-w", &files[1], "-g", "enrollment", "-m", "0664"
    ]))
}

pub fn set_iotedge_gateway_config(config_file: &PathBuf, image_file: &PathBuf, root_ca_file: &PathBuf, edge_device_identity_full_chain_file: &PathBuf, edge_device_identity_key_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (binds, files) = prepare_binds(vec![
        config_file,
        edge_device_identity_full_chain_file,
        edge_device_identity_key_file,
        root_ca_file,
        image_file
    ])?;

    docker_exec(Some(binds), Some(vec![
        "set_iotedge_gw_config.sh", "-c", &files[0], "-e", &files[1], "-k", &files[2], "-r", &files[3], "-w", &files[4]
    ]))
}

pub fn set_iot_leaf_sas_config(config_file: &PathBuf, image_file: &PathBuf, root_ca_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (binds, files) = prepare_binds(vec![
        config_file,
        root_ca_file,
        image_file
    ])?;

    docker_exec(Some(binds), Some(vec!["set_iot_leaf_config.sh", "-c", &files[0], "-r", &files[1], "-w", &files[2]]))
}

pub fn set_identity_config(config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (binds, files) = prepare_binds(vec![config_file, image_file])?;

    docker_exec(Some(binds), Some(vec!["set_identity_config.sh", "-c", &files[0], "-w", &files[1]]))
}

pub fn set_iot_hub_device_update_config(config_file: &PathBuf, image_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (binds, files) = prepare_binds(vec![config_file, image_file])?;

    docker_exec(Some(binds), Some(vec![
        "copy_file_to_image.sh", "-i", &files[0], "-o", "upper/adu/adu-conf.txt", "-p", "etc", "-w", &files[1], "-g", "adu", "-u", "adu", "-m", "0664"
    ]))
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

pub fn prepare_binds(files: Vec<&PathBuf>) -> Result<(Vec<String>, Vec<String>), Box<dyn std::error::Error>> {
    let mut binds: Vec<String> = vec![];
    let mut bind_files: Vec<String> = vec![];
    let tmp_folder = Uuid::new_v4();

    files.iter().try_for_each(|&f| -> Result<(), Error>{
        let path = ensure_filepath(&f)?;
        let bind_path = format!("/tmp/{}/{}", tmp_folder, Uuid::new_v4());
        bind_files.push(bind_path.clone());
        binds.push(format!("{}:{}", path, bind_path));
        Ok(())
    })?;
    
    Ok((binds, bind_files))
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
