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
        Command::Identity(IdentityConfig::Set{config,image}) => {res = identity::config(config, image)},
    };

    return res;
}
