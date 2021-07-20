use structopt::StructOpt;

const ABOUT: &'static str = "This tools helps to manage your ics-dm devices. For more information visit:\nhttps://github.com/ICS-DeviceManagement/ics-dm-cli";
const COPYRIGHT: &'static str = "Copyright © 2021 by conplement AG";

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum IdentityConfig {
    SetConfig {
        /// path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
        /// path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf
    },
    SetIotedgeGatewayConfig {
        /// path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
        /// path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
        /// path to root ca certificate file
        #[structopt(short = "r", long = "root_ca")]
        #[structopt(parse(from_os_str))]
        root_ca: std::path::PathBuf,
        /// path to device identity certificate file
        #[structopt(short = "d", long = "device_identity")]
        #[structopt(parse(from_os_str))]
        device_identity: std::path::PathBuf,
        /// path to device identity certificate key file
        #[structopt(short = "k", long = "device_identity_key")]
        #[structopt(parse(from_os_str))]
        device_identity_key: std::path::PathBuf,
    },
    SetIotLeafSasConfig {
        /// path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
        /// path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
        /// path to root ca certificate file
        #[structopt(short = "r", long = "root_ca")]
        #[structopt(parse(from_os_str))]
        root_ca: std::path::PathBuf,
    },
    Info {
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    }
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum WifiConfig {
    Set {
        /// path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
         /// path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    },
    Info {
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    }
}

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum EnrollmentConfig {
    Set {
        /// path to config file
        #[structopt(short = "c", long = "enrollment-config")]
        #[structopt(parse(from_os_str))]
        enrollment_config: std::path::PathBuf,
         /// path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    },
    Info {
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
    }
}


#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum Command {
    Identity(IdentityConfig),
    DockerInfo,
    Wifi(WifiConfig),
    Enrollment(EnrollmentConfig)
}

pub fn from_args() -> Command { Command::from_args() }
