mod cli;
mod docker;
pub mod file;
mod identity;
mod wifi;

use std::error::Error;

use cli::Command;
use cli::WifiConfig::Set;
use cli::WifiConfig::Info as WifiInfo;
use cli::IdentityConfig::Info as IdentityInfo;
use cli::IdentityConfig::SetIotedgeGatewayConfig;
use cli::IdentityConfig::SetIotedgeLeafSasConfig;

pub fn run() -> Result<(), Box<dyn Error>> {
    match cli::from_args() {
        Command::DockerInfo => docker::docker_version()?,
        Command::Wifi(WifiInfo {image}) => wifi::info(image)?,
        Command::Wifi(Set {config, image}) => wifi::config(config, image)?,
        Command::Identity(IdentityInfo{image}) => identity::info(image)?,
        Command::Identity(SetIotedgeGatewayConfig{config,image,root_ca,device_identity,device_identity_key}) => identity::set_iotedge_gateway_config(config,image,root_ca,device_identity,device_identity_key)?,
        Command::Identity(SetIotedgeLeafSasConfig{config,image,root_ca}) => identity::set_iotedge_sas_leaf_config(config,image,root_ca)?,

    }

    Ok(())
}