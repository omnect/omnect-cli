use super::cli::Partition;
use super::validators::identity::{validate_identity, IdentityType};
use super::validators::ssh::validate_ssh_pub_key;
use anyhow::{Context, Result};
use log::warn;
use path_absolutize::Absolutize;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};
use uuid::Uuid;

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
    // ToDo: as soon as we get rid of docker create temp file under /tmp/
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

    //docker_exec(Some(binds), cmdline.iter().map(AsRef::as_ref).collect())
    Ok(())
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