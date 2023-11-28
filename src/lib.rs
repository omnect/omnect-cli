#[macro_use]
extern crate lazy_static;

pub mod auth;
pub mod cli;
pub mod file;
pub mod ssh;
mod validators;
use anyhow::{Context, Result};
use cli::Command;
use cli::FileConfig::{CopyFromImage, CopyToImage};
use cli::IdentityConfig::SetConfig;
use cli::IdentityConfig::SetDeviceCertificate;
use cli::IdentityConfig::SetIotLeafSasConfig;
use cli::IdentityConfig::SetIotedgeGatewayConfig;
use cli::IdentityConfig::SetSshTunnelCertificate;
use cli::IotHubDeviceUpdateConfig::Set as IotHubDeviceUpdateSet;
use cli::SshConfig;
use std::path::PathBuf;
use uuid::Uuid;

fn run_image_command<F>(image_file: PathBuf, generate_bmap: bool, f: F) -> Result<()>
where
    F: FnOnce(&PathBuf) -> Result<()>,
{
    anyhow::ensure!(
        image_file.exists(),
        "image doesn't exist: {}",
        image_file.to_str().unwrap()
    );

    // copy image to /tmp/{uuid}
    let mut tmp_image_file = PathBuf::from(format!("/tmp/{}", Uuid::new_v4()));
    tmp_image_file.push(image_file.file_name().unwrap());
    std::fs::copy(&image_file, &tmp_image_file)?;

    // run command
    f(&tmp_image_file)?;

    // copy back bmap file if one was created
    if generate_bmap {
        std::fs::copy(
            &format!("{}.bmap", tmp_image_file.to_str().unwrap()),
            format!("{}.bmap", image_file.to_str().unwrap()),
        )?;
    }

    // copy back image file
    std::fs::copy(&tmp_image_file, &image_file)?;
    Ok(())
}

pub fn run() -> Result<()> {
    match cli::from_args() {
        Command::Identity(SetConfig {
            config,
            image,
            payload,
            generate_bmap,
        }) => run_image_command(image, generate_bmap, |img| {
            file::set_identity_config(&config, &img, generate_bmap, payload)
        })?,
        Command::Identity(SetDeviceCertificate {
            intermediate_full_chain_cert,
            intermediate_key,
            image,
            device_id,
            days,
            generate_bmap,
        }) => {
            // ToDo move to mod::set_device_cert
            let intermediate_full_chain_cert_str =
                std::fs::read_to_string(&intermediate_full_chain_cert)?;
            let intermediate_key_str = std::fs::read_to_string(intermediate_key)?;
            let crypto = omnect_crypto::Crypto::new(
                intermediate_key_str.as_bytes(),
                intermediate_full_chain_cert_str.as_bytes(),
            )?;
            let (device_cert_pem, device_key_pem) =
                crypto.create_cert_and_key(&device_id, &None, days)?;

            run_image_command(image, generate_bmap, |img| {
                file::set_device_cert(
                    &intermediate_full_chain_cert,
                    &device_cert_pem,
                    &device_key_pem,
                    &img,
                    generate_bmap,
                )
            })?
        }
        Command::Identity(SetIotedgeGatewayConfig {
            config,
            image,
            root_ca,
            device_identity,
            device_identity_key,
            generate_bmap,
        }) => run_image_command(image, generate_bmap, |img: &PathBuf| {
            file::set_iotedge_gateway_config(
                &config,
                &img,
                &root_ca,
                &device_identity,
                &device_identity_key,
                generate_bmap,
            )
        })?,
        Command::Identity(SetIotLeafSasConfig {
            config,
            image,
            root_ca,
            generate_bmap,
        }) => run_image_command(image, generate_bmap, |img: &PathBuf| {
            file::set_iot_leaf_sas_config(&config, &img, &root_ca, generate_bmap)
        })?,
        Command::Identity(SetSshTunnelCertificate {
            image,
            root_ca,
            device_principal,
            generate_bmap,
        }) => run_image_command(image, generate_bmap, |img: &PathBuf| {
            file::set_ssh_tunnel_certificate(&img, &root_ca, &device_principal, generate_bmap)
        })?,
        Command::IotHubDeviceUpdate(IotHubDeviceUpdateSet {
            iot_hub_device_update_config,
            image,
            generate_bmap,
        }) => run_image_command(image, generate_bmap, |img: &PathBuf| {
            file::set_iot_hub_device_update_config(
                &iot_hub_device_update_config,
                &img,
                generate_bmap,
            )
        })?,
        Command::Ssh(SshConfig {
            device,
            username,
            dir,
            priv_key_path,
            config_path,
            backend,
        }) => {
            #[tokio::main]
            async fn create_ssh_tunnel(
                device: &str,
                username: &str,
                dir: Option<PathBuf>,
                priv_key_path: Option<PathBuf>,
                config_path: Option<PathBuf>,
                backend: String,
            ) -> Result<()> {
                let access_token = crate::auth::authorize(&*crate::auth::AUTH_INFO_DEV)
                    .await
                    .context("create ssh tunnel")?;

                let config = ssh::Config::new(backend, dir, priv_key_path, config_path)?;

                ssh::ssh_create_tunnel(device, username, config, access_token).await
            }

            create_ssh_tunnel(&device, &username, dir, priv_key_path, config_path, backend)?;
        }
        Command::File(CopyToImage {
            file_copy_params,
            image,
            generate_bmap,
        }) => run_image_command(image, generate_bmap, |img: &PathBuf| {
            file::copy_to_image(&file_copy_params, &img, generate_bmap)
        })?,
        Command::File(CopyFromImage {
            file_copy_params,
            image,
        }) => run_image_command(image, false, |img: &PathBuf| {
            file::copy_from_image(&file_copy_params, &img)})?,
    }

    Ok(())
}
