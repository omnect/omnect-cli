#[macro_use]
extern crate lazy_static;

mod cli;
use ics_dm_crypto;
pub mod docker;
mod validators;

use cli::BootConfig::Set as BootSet;
use cli::Command;
use cli::EnrollmentConfig::Set as EnrollmentSet;
use cli::IdentityConfig::SetConfig;
use cli::IdentityConfig::SetDeviceCertificate;
use cli::IdentityConfig::SetIotLeafSasConfig;
use cli::IdentityConfig::SetIotedgeGatewayConfig;
use cli::IotHubDeviceUpdateConfig::Set as IotHubDeviceUpdateSet;
use cli::WifiConfig::Set as WifiSet;
use std::io::{Error, ErrorKind};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    match cli::from_args() {
        Command::DockerInfo => docker::docker_version()?,
        Command::Wifi(WifiSet {
            config,
            image,
            generate_bmap,
        }) => docker::set_wifi_config(&config, &image, generate_bmap)?,
        Command::Enrollment(EnrollmentSet {
            enrollment_config,
            image,
            generate_bmap,
        }) => docker::set_enrollment_config(&enrollment_config, &image, generate_bmap)?,
        Command::Identity(SetConfig {
            config,
            image,
            generate_bmap,
        }) => docker::set_identity_config(&config, &image, generate_bmap)?,
        Command::Identity(SetDeviceCertificate {
            intermediate_full_chain_cert,
            intermediate_key,
            image,
            device_id,
            days,
            generate_bmap,
        }) => {
            let intermediate_full_chain_cert_str =
                std::fs::read_to_string(&intermediate_full_chain_cert)?;
            let intermediate_key_str = std::fs::read_to_string(intermediate_key)?;
            let crypto = ics_dm_crypto::Crypto::new(
                intermediate_key_str.as_bytes(),
                intermediate_full_chain_cert_str.as_bytes(),
            )?;
            let (device_cert_pem, device_key_pem) =
                crypto.create_cert_and_key(&device_id, &None, days)?;
            docker::set_device_cert(
                &intermediate_full_chain_cert,
                &device_cert_pem,
                &device_key_pem,
                &image,
                generate_bmap,
            )?
        }
        Command::Identity(SetIotedgeGatewayConfig {
            config,
            image,
            root_ca,
            device_identity,
            device_identity_key,
            generate_bmap,
        }) => docker::set_iotedge_gateway_config(
            &config,
            &image,
            &root_ca,
            &device_identity,
            &device_identity_key,
            generate_bmap,
        )?,
        Command::Identity(SetIotLeafSasConfig {
            config,
            image,
            root_ca,
            generate_bmap,
        }) => docker::set_iot_leaf_sas_config(&config, &image, &root_ca, generate_bmap)?,
        Command::IotHubDeviceUpdate(IotHubDeviceUpdateSet {
            iot_hub_device_update_config,
            image,
            generate_bmap,
        }) => docker::set_iot_hub_device_update_config(
            &iot_hub_device_update_config,
            &image,
            generate_bmap,
        )?,
        Command::Boot(BootSet {
            boot_script,
            image,
            generate_bmap,
        }) => docker::set_boot_config(&boot_script, &image, generate_bmap)?,
        _ => Err(Error::new(ErrorKind::Other, "Not implemented"))?,
    }

    Ok(())
}
