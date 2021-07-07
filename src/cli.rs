use structopt::StructOpt;

const ABOUT: &'static str = "- manage your ics-dm devices";
const COPYRIGHT: &'static str = "© 2021 conplement AG";

#[derive(StructOpt, Debug, PartialEq)]
#[structopt(about = ABOUT)]
#[structopt(after_help = COPYRIGHT)]
pub enum IdentityConfig {
    SetIotedgeGatewayConfig {
        /// absolute path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
        /// absolute path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
        /// absolute path to root ca certificate file
        #[structopt(short = "r", long = "root_ca")]
        #[structopt(parse(from_os_str))]
        root_ca: std::path::PathBuf,
        /// absolute path to device identity certificate file
        #[structopt(short = "d", long = "device_identity")]
        #[structopt(parse(from_os_str))]
        device_identity: std::path::PathBuf,
        /// absolute path to device identity certificate key file
        #[structopt(short = "k", long = "device_identity_key")]
        #[structopt(parse(from_os_str))]
        device_identity_key: std::path::PathBuf,
    },
    SetIotedgeLeafSasConfig {
        /// absolute path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
        /// absolute path to uncompressed image file
        #[structopt(short = "i", long = "image")]
        #[structopt(parse(from_os_str))]
        image: std::path::PathBuf,
        /// absolute path to root ca certificate file
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
        /// absolute path to config file
        #[structopt(short = "c", long = "config")]
        #[structopt(parse(from_os_str))]
        config: std::path::PathBuf,
         /// absolute path to uncompressed image file
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
        /// absolute path to config file
        #[structopt(short = "ec", long = "enrollment-config")]
        #[structopt(parse(from_os_str))]
        enrollment_config: std::path::PathBuf,
        /// absolute path to config file
        #[structopt(short = "pc", long = "provisioning-config")]
        #[structopt(parse(from_os_str))]
        provisioning_config: std::path::PathBuf,
         /// absolute path to uncompressed image file
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
