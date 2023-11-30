pub mod compression;
pub mod functions;
use super::validators::{
    device_update,
    identity::{validate_identity, IdentityType},
    ssh::validate_ssh_pub_key,
};
use crate::file::functions::{FileCopyFromParams, FileCopyToParams, Partition};
use anyhow::{Context, Result};
use log::warn;
use std::fs;
use std::path::{Path, PathBuf};

pub fn set_iotedge_gateway_config(
    config_file: &Path,
    image_file: &Path,
    root_ca_file: &Path,
    edge_device_identity_full_chain_file: &Path,
    edge_device_identity_key_file: &Path,
    generate_bmap: bool,
) -> Result<()> {
    validate_identity(IdentityType::Gateway, config_file, &None)?
        .iter()
        .for_each(|x| warn!("{}", x));

    copy_to_image(
        &vec![
            FileCopyToParams::new(
                &config_file.to_path_buf(),
                Partition::factory,
                &PathBuf::from("/etc/aziot/config.toml"),
            ),
            FileCopyToParams::new(
                &root_ca_file.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/ca/trust-bundle.pem.crt"),
            ),
            FileCopyToParams::new(
                &edge_device_identity_full_chain_file.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/priv/edge-ca.pem"),
            ),
            FileCopyToParams::new(
                &edge_device_identity_key_file.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/priv/edge-ca.key.pem"),
            ),
        ],
        image_file,
        generate_bmap,
    )
}

pub fn set_iot_leaf_sas_config(
    config_file: &Path,
    image_file: &Path,
    root_ca_file: &Path,
    generate_bmap: bool,
) -> Result<()> {
    validate_identity(IdentityType::Leaf, config_file, &None)?
        .iter()
        .for_each(|x| warn!("{}", x));

    let mut root_ca_out_file = PathBuf::from("ca");
    root_ca_out_file.push(root_ca_file.file_name().unwrap());
    root_ca_out_file.set_extension("crt");

    copy_to_image(
        &[
            FileCopyToParams::new(
                &config_file.to_path_buf(),
                Partition::factory,
                &PathBuf::from("/etc/aziot/config.toml"),
            ),
            FileCopyToParams::new(
                &root_ca_file.to_path_buf(),
                Partition::cert,
                &root_ca_out_file,
            ),
        ],
        image_file,
        generate_bmap,
    )
}

pub fn set_ssh_tunnel_certificate(
    image_file: &Path,
    root_ca_file: &Path,
    device_principal: &str,
    generate_bmap: bool,
) -> Result<()> {
    validate_ssh_pub_key(root_ca_file)?;

    // we use the folder the image is located in
    // the caller is responsible to create a /tmp/ directory if needed
    let mut authorized_principals_file = image_file
        .parent()
        .context("copy_to_image: cannot get directory of image")?
        .to_path_buf();

    authorized_principals_file.push("authorized_principals");
    fs::write(&authorized_principals_file, device_principal)?;

    copy_to_image(
        &[
            FileCopyToParams::new(
                &root_ca_file.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/ssh/root_ca"),
            ),
            FileCopyToParams::new(
                &authorized_principals_file.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/ssh/authorized_principals"),
            ),
        ],
        image_file,
        generate_bmap,
    )
}

pub fn set_identity_config(
    config_file: &Path,
    image_file: &Path,
    generate_bmap: bool,
    payload: Option<&Path>,
) -> Result<()> {
    validate_identity(IdentityType::Standalone, config_file, &payload)?
        .iter()
        .for_each(|x| warn!("{}", x));

    let mut files = vec![FileCopyToParams::new(
        &config_file.to_path_buf(),
        Partition::factory,
        &PathBuf::from("/etc/aziot/config.toml"),
    )];

    if let Some(p) = payload {
        files.push(FileCopyToParams::new(
            &p.to_path_buf(),
            Partition::factory,
            &PathBuf::from("/etc/omnect/dps-payload.json"),
        ));
    }

    copy_to_image(&files, image_file, generate_bmap)
}

pub fn set_device_cert(
    intermediate_full_chain_cert_path: &Path,
    device_full_chain_cert: &Vec<u8>,
    device_key: &Vec<u8>,
    image_file: &Path,
    generate_bmap: bool,
) -> Result<()> {
    // we use the folder the image is located in
    // the caller is responsible to create a /tmp/ directory if needed
    let mut device_cert_path = image_file
        .parent()
        .context("copy_to_image: cannot get directory of image")?
        .to_path_buf();
    let mut device_key_path = device_cert_path.clone();

    device_cert_path.push("device_cert_path.pem");
    device_key_path.push("device_key_path.key.pem");

    fs::write(&device_cert_path, device_full_chain_cert)
        .context("set_device_cert: write device_cert_path")?;
    fs::write(&device_key_path, device_key).context("set_device_cert: write device_key_path")?;

    copy_to_image(
        &vec![
            FileCopyToParams::new(
                &device_cert_path,
                Partition::cert,
                &PathBuf::from("/priv/device_id_cert.pem"),
            ),
            FileCopyToParams::new(
                &device_key_path,
                Partition::cert,
                &PathBuf::from("/priv/device_id_cert_key.pem"),
            ),
            FileCopyToParams::new(
                &intermediate_full_chain_cert_path.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/priv/ca.crt.pem"),
            ),
            FileCopyToParams::new(
                &intermediate_full_chain_cert_path.to_path_buf(),
                Partition::cert,
                &PathBuf::from("/ca/ca.crt"),
            ),
        ],
        image_file,
        generate_bmap,
    )
}

pub fn set_iot_hub_device_update_config(
    du_config_file: &Path,
    image_file: &Path,
    generate_bmap: bool,
) -> Result<()> {
    device_update::validate_config(&du_config_file)?;

    copy_to_image(
        &[FileCopyToParams::new(
            &du_config_file.to_path_buf(),
            Partition::factory,
            &PathBuf::from("/etc/adu/du-config.json"),
        )],
        image_file,
        generate_bmap,
    )
}

pub fn copy_to_image(
    file_copy_params: &[FileCopyToParams],
    image_file: &Path,
    generate_bmap: bool,
) -> Result<()> {
    functions::copy_to_image(file_copy_params, image_file, generate_bmap)
}

pub fn copy_from_image(file_copy_params: &[FileCopyFromParams], image_file: &Path) -> Result<()> {
    functions::copy_from_image(file_copy_params, image_file)
}
