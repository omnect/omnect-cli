pub mod compression;
pub mod functions;
use super::validators::{
    device_update,
    identity::{validate_identity, IdentityConfig, IdentityType},
    ssh::validate_ssh_pub_key,
};
use crate::file::functions::{FileCopyFromParams, FileCopyToParams, Partition};
use anyhow::{Context, Result};
use log::warn;
use regex::Regex;
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

    let mut file_copies = configure_hostname(config_file, image_file)?;
    file_copies.append(&mut vec![
        FileCopyToParams::new(
            config_file,
            Partition::factory,
            Path::new("/etc/aziot/config.toml"),
        ),
        FileCopyToParams::new(
            root_ca_file,
            Partition::cert,
            Path::new("/ca/trust-bundle.pem.crt"),
        ),
        FileCopyToParams::new(
            edge_device_identity_full_chain_file,
            Partition::cert,
            Path::new("/priv/edge-ca.pem"),
        ),
        FileCopyToParams::new(
            edge_device_identity_key_file,
            Partition::cert,
            Path::new("/priv/edge-ca.key.pem"),
        ),
    ]);

    copy_to_image(&file_copies, image_file, generate_bmap)
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

    let mut file_copies = configure_hostname(config_file, image_file)?;
    file_copies.append(&mut vec![
        FileCopyToParams::new(
            config_file,
            Partition::factory,
            Path::new("/etc/aziot/config.toml"),
        ),
        FileCopyToParams::new(root_ca_file, Partition::cert, &root_ca_out_file),
    ]);

    copy_to_image(&file_copies, image_file, generate_bmap)
}

pub fn set_ssh_tunnel_certificate(
    image_file: &Path,
    root_ca_file: &Path,
    device_principal: &str,
    generate_bmap: bool,
) -> Result<()> {
    validate_ssh_pub_key(root_ca_file)?;
    let authorized_principals_file = get_file_path(image_file.parent(), "authorized_principals")?;
    fs::write(&authorized_principals_file, device_principal)?;

    copy_to_image(
        &[
            FileCopyToParams::new(root_ca_file, Partition::cert, Path::new("/ssh/root_ca")),
            FileCopyToParams::new(
                &authorized_principals_file.to_path_buf(),
                Partition::cert,
                Path::new("/ssh/authorized_principals"),
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

    let mut file_copies = configure_hostname(config_file, image_file)?;
    file_copies.append(&mut vec![FileCopyToParams::new(
        config_file,
        Partition::factory,
        Path::new("/etc/aziot/config.toml"),
    )]);

    if let Some(p) = payload {
        file_copies.push(FileCopyToParams::new(
            p,
            Partition::factory,
            Path::new("/etc/omnect/dps-payload.json"),
        ));
    }
    copy_to_image(&file_copies, image_file, generate_bmap)
}

pub fn set_device_cert(
    intermediate_full_chain_cert_path: &Path,
    device_full_chain_cert: &Vec<u8>,
    device_key: &Vec<u8>,
    image_file: &Path,
    generate_bmap: bool,
) -> Result<()> {
    let parent = image_file.parent();
    let device_cert_path = get_file_path(parent, "device_cert_path.pem")?;
    let device_key_path = get_file_path(parent, "device_key_path.key.pem")?;

    fs::write(&device_cert_path, device_full_chain_cert)
        .context("set_device_cert: write device_cert_path")?;
    fs::write(&device_key_path, device_key).context("set_device_cert: write device_key_path")?;

    copy_to_image(
        &vec![
            FileCopyToParams::new(
                &device_cert_path,
                Partition::cert,
                Path::new("/priv/device_id_cert.pem"),
            ),
            FileCopyToParams::new(
                &device_key_path,
                Partition::cert,
                Path::new("/priv/device_id_cert_key.pem"),
            ),
            FileCopyToParams::new(
                intermediate_full_chain_cert_path,
                Partition::cert,
                Path::new("/priv/ca.crt.pem"),
            ),
            FileCopyToParams::new(
                intermediate_full_chain_cert_path,
                Partition::cert,
                Path::new("/ca/ca.crt"),
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
    device_update::validate_config(du_config_file)?;

    copy_to_image(
        &[FileCopyToParams::new(
            du_config_file,
            Partition::factory,
            Path::new("/etc/adu/du-config.json"),
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

fn configure_hostname(
    identity_config_file: &Path,
    image_file: &Path,
) -> Result<Vec<FileCopyToParams>> {
    let hostname_file = get_file_path(image_file.parent(), "hostname")?;
    let hosts_file = get_file_path(image_file.parent(), "hosts")?;

    // get hostname from identity_config_file
    let identity: IdentityConfig = serde_path_to_error::deserialize(&mut toml::Deserializer::new(
        fs::read_to_string(identity_config_file.to_str().unwrap())
            .context("configure_hostname: cannot read identity file")?
            .as_str(),
    ))
    .context("configure_hostname: couldn't read identity")?;

    fs::write(&hostname_file, &identity.hostname)
        .context("configure_hostname: cannot write to hostname file")?;

    // read /etc/hosts from rootA
    copy_from_image(
        &[FileCopyFromParams::new(
            Path::new("/etc/hosts"),
            Partition::rootA,
            &hosts_file.to_path_buf(),
        )],
        image_file,
    )
    .context("configure_hostname: couldn't read /etc/hosts from rootA")?;

    // patch /etc/hosts with hostname
    let content =
        fs::read_to_string(&hosts_file).context("configure_hostname: cannot read hosts file")?;

    let reg =
        Regex::new(r"(127\.0\.1\.1.*)").context("configure_hostname: create hostname regex")?;

    let content = reg.replace_all(content.as_str(), format!("127.0.1.1 {}", identity.hostname));

    fs::write(&hosts_file, content.to_string())
        .context("configure_hostname: cannot write to hosts file")?;

    Ok(vec![
        FileCopyToParams::new(
            &hostname_file.to_path_buf(),
            Partition::factory,
            Path::new("/etc/hostname"),
        ),
        FileCopyToParams::new(
            &hosts_file.to_path_buf(),
            Partition::factory,
            Path::new("/etc/hosts"),
        ),
    ])
}

fn get_file_path(parent: Option<&Path>, file_name: &str) -> Result<PathBuf> {
    let mut file_path = parent
        .context("get_file_path: cannot get parent directory")?
        .to_path_buf();
    file_path.push(file_name);
    Ok(file_path)
}
