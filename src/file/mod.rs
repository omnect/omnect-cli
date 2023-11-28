pub mod functions;
use super::validators::identity::{validate_identity, IdentityType};
use super::validators::ssh::validate_ssh_pub_key;
use crate::file::functions::{FileCopyFromParams, FileCopyToParams, Partition};
use anyhow::{Context, Result};
use log::warn;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn set_iotedge_gateway_config(
    config_file: &PathBuf,
    _image_file: &PathBuf,
    _root_ca_file: &PathBuf,
    _edge_device_identity_full_chain_file: &PathBuf,
    _edge_device_identity_key_file: &PathBuf,
    _generate_bmap: bool,
) -> Result<()> {
    validate_identity(IdentityType::Gateway, config_file, &None)?
        .iter()
        .for_each(|x| warn!("{}", x));
    Ok(())
/* 
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
                generate_bmap,
            )
        },
    ) */
}

pub fn set_iot_leaf_sas_config(
    config_file: &PathBuf,
    _image_file: &PathBuf,
    _root_ca_file: &PathBuf,
    _generate_bmap: bool,
) -> Result<()> {
    validate_identity(IdentityType::Leaf, config_file, &None)?
        .iter()
        .for_each(|x| warn!("{}", x));
    Ok(())
/* 
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
                generate_bmap,
            )
        },
    ) */
}

pub fn set_ssh_tunnel_certificate(
    _image_file: &PathBuf,
    root_ca_file: &PathBuf,
    _device_principal: &str,
    _generate_bmap: bool,
) -> Result<()> {
    validate_ssh_pub_key(root_ca_file)?;
    Ok(())
/* 
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
                generate_bmap,
            )
        },
    ) */
}

pub fn set_identity_config(
    config_file: &PathBuf,
    _image_file: &PathBuf,
    _generate_bmap: bool,
    payload: Option<PathBuf>,
) -> Result<()> {
    validate_identity(IdentityType::Standalone, config_file, &payload)?
        .iter()
        .for_each(|x| warn!("{}", x));
    Ok(())
/* 
    super::validators::image::image_action(
        image_file,
        true,
        move |image_file: &PathBuf| -> Result<()> {
            if let Some(payload) = payload {
                cmd_exec(
                    vec![&payload, image_file],
                    |files| -> String {
                        format!(
                            "copy_file_to_image.sh, -i, {0}, -o, /etc/omnect/dps-payload.json, -p, factory, -w {1}",
                            files[0], files[1]
                        )
                    },
                    false,
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
                generate_bmap,
            )
        },
    ) */
}

pub fn set_device_cert(
    _intermediate_full_chain_cert_path: &PathBuf,
    _device_full_chain_cert: &Vec<u8>,
    _device_key: &Vec<u8>,
    _image_file: &PathBuf,
    _generate_bmap: bool,
) -> Result<()> {
    /*
    let uuid = Uuid::new_v4();
    let device_cert_path = PathBuf::from(format!("/tmp/{}.pem", uuid));
    let device_key_path = PathBuf::from(format!("/tmp/{}.key.pem", uuid));

    fs::write(device_cert_path, device_full_chain_cert)
        .context("set_device_cert: write device_cert_path")?;
    fs::write(device_key_path, device_key).context("set_device_cert: write device_key_path")?;

    copy_to_image(
        &vec![
            FileCopyToParams::new(
                device_cert_path,
                Partition::cert,
                PathBuf::from("/priv/device_id_cert.pem"),
            ),
            FileCopyToParams::new(
                device_key_path,
                Partition::cert,
                PathBuf::from("/priv/device_id_cert_key.pem"),
            ),
            FileCopyToParams::new(
                intermediate_full_chain_cert_path,
                Partition::cert,
                PathBuf::from("/priv/ca.crt.pem"),
            ),
            FileCopyToParams::new(
                intermediate_full_chain_cert_path,
                Partition::cert,
                PathBuf::from("/ca/ca.crt"),
            ),
        ],
        image_file,
        generate_bmap,
    )
        "copy_file_to_image.sh, -i, {0}, -o, /priv/device_id_cert.pem, -p, cert, -w {1}",
        "copy_file_to_image.sh, -i, {0}, -o, /priv/device_id_cert_key.pem, -p, cert, -w {1}",
        "copy_file_to_image.sh, -i, {0}, -o, /priv/ca.crt.pem, -p, cert, -w {1}",
        "copy_file_to_image.sh, -i, {0}, -o, /ca/ca.crt, -p, cert, -w {1}",
    */
    Ok(())
}

pub fn set_iot_hub_device_update_config(
    in_file: &PathBuf,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<()> {
    // ToDo validate du-config json
    /*     let file =
            File::open(in_file).context("set_iot_hub_device_update_config: open config_file")?;
        serde_json::from_reader::<_, serde_json::Value>(BufReader::new(file))
            .context("set_iot_hub_device_update_config: read config_file")?;
    */

    copy_to_image(
        &vec![FileCopyToParams::new(
            in_file.clone(),
            Partition::factory,
            PathBuf::from("/etc/adu/du-config.json"),
        )],
        image_file,
        generate_bmap,
    )
}

pub fn copy_to_image(
    file_copy_params: &Vec<FileCopyToParams>,
    image_file: &PathBuf,
    generate_bmap: bool,
) -> Result<()> {
    functions::copy_to_image(file_copy_params, image_file, generate_bmap)
}

pub fn copy_from_image(
    file_copy_params: &Vec<FileCopyFromParams>,
    image_file: &PathBuf,
) -> Result<()> {
    functions::copy_from_image(file_copy_params, image_file)
}
