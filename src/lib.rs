#[macro_use]
extern crate lazy_static;

mod cli;
pub mod docker;

use std::io::{Error, ErrorKind};

use cli::Command;
use cli::WifiConfig::Set as WifiSet;
use cli::EnrollmentConfig::Set as EnrollmentSet;
use cli::IdentityConfig::SetConfig;
use cli::IdentityConfig::SetIotedgeGatewayConfig;
use cli::IdentityConfig::SetIotLeafSasConfig;
use cli::IotHubDeviceUpdateConfig::Set as IotHubDeviceUpdateSet;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    match cli::from_args() {
        Command::DockerInfo =>
        {
            docker::docker_version()?
        },
        Command::Wifi(WifiSet {config, image}) =>
        {
            docker::set_wifi_config(&config, &image)?
        },
        Command::Enrollment(EnrollmentSet {enrollment_config, image}) =>
        {
            docker::set_enrollment_config(&enrollment_config, &image)?
        },
        Command::Identity(SetConfig{config, image}) =>
        {
            docker::set_identity_config(&config, &image)?
        },
        Command::Identity(SetIotedgeGatewayConfig{config, image, root_ca, device_identity, device_identity_key}) =>
        {
            docker::set_iotedge_gateway_config(&config, &image, &root_ca, &device_identity, &device_identity_key)?
        },
        Command::Identity(SetIotLeafSasConfig{config, image, root_ca}) =>
        {
            docker::set_iot_leaf_sas_config(&config, &image, &root_ca)?
        },
        Command::IotHubDeviceUpdate(IotHubDeviceUpdateSet{ iot_hub_device_update_config, image}) =>
        {
            docker::set_iot_hub_device_update_config(&iot_hub_device_update_config, &image)?
        }
        _ =>
        {
            Err(Error::new(ErrorKind::Other, "Not implemented"))?
        }
    }

    Ok(())
}
