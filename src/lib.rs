#[macro_use]
extern crate lazy_static;

mod cli;
pub mod docker;
mod validators;

use cli::Command;
use cli::EnrollmentConfig::Set as EnrollmentSet;
use cli::IdentityConfig::SetConfig;
use cli::IdentityConfig::SetIotLeafSasConfig;
use cli::IdentityConfig::SetIotedgeGatewayConfig;
use cli::IotHubDeviceUpdateConfig::Set as IotHubDeviceUpdateSet;
use cli::BootConfig::Set as BootSet;
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
        }) => docker::set_boot_config(
            &boot_script,
            &image,
            generate_bmap,
        )?,
        _ => Err(Error::new(ErrorKind::Other, "Not implemented"))?,
    }

    Ok(())
}
