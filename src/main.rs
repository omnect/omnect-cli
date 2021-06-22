mod cli;
mod docker;
mod file;
mod identity;
mod wifi;

use cli::Command;
use cli::WifiConfig;
use cli::IdentityConfig;

fn main() -> Result<(), std::io::Error> {
    let res;
    match cli::from_args() {
        Command::DockerInfo => {res = docker::docker_version();},
        Command::Wifi(WifiConfig::Info {image}) => {res = wifi::info(image)},
        Command::Wifi(WifiConfig::Set {config, image}) => {res = wifi::config(config, image)},
        Command::Identity(IdentityConfig::Info{image}) => {res = identity::info(image)},
        Command::Identity(IdentityConfig::SetIotedgeGatewayConfig{config,image,root_ca,device_identity,device_identity_key}) => {res = identity::set_iotedge_gateway_config(config,image,root_ca,device_identity,device_identity_key)},
        Command::Identity(IdentityConfig::SetIotedgeLeafSasConfig{config,image,root_ca}) => {res = identity::set_iotedge_sas_leaf_config(config,image,root_ca)},
    };

    return res;
}
