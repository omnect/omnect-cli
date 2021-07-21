mod cli;
pub mod docker;

use std::io::{Error, ErrorKind};

use cli::Command;
use cli::WifiConfig::Set as WifiSet;
use cli::EnrollmentConfig::Set as EnrollmentSet;
use cli::IdentityConfig::SetConfig;
use cli::IdentityConfig::SetIotedgeGatewayConfig;
use cli::IdentityConfig::SetIotLeafSasConfig;

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
        _ => 
        {
            Err(Error::new(ErrorKind::Other, "Not implemented"))?
        }
    }

    Ok(())
}