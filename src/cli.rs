use clap::Parser;

#[derive(Parser, Debug)]
#[command(after_help="Copyright © 2021 by conplement AG")]
/// pre-configure device identity settings
pub enum IdentityConfig {
    /// set general config.toml file
    SetConfig {
        /// path to config.toml file
        #[arg(short = 'c', long = "config")]
        config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,

        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
    /// set transparent gateway config.toml file and additional certificates and keys
    SetIotedgeGatewayConfig {
        /// path to config.toml file
        #[arg(short = 'c', long = "config")]
        config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// path to root ca certificate file
        #[arg(short = 'r', long = "root_ca")]
        root_ca: std::path::PathBuf,
        /// path to device identity certificate file
        #[arg(short = 'd', long = "device_identity")]
        device_identity: std::path::PathBuf,
        /// path to device identity certificate key file
        #[arg(short = 'k', long = "device_identity_key")]
        device_identity_key: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
    /// set leaf device config.toml file and additional certificate
    SetIotLeafSasConfig {
        /// path to config.toml file
        #[arg(short = 'c', long = "config")]
        config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// path to root ca certificate file
        #[arg(short = 'r', long = "root_ca")]
        root_ca: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
    /// set certificates in order to support X.509 based DPS provisioning and certificate renewal via EST
    SetDeviceCertificate {
        /// path to intermediate full-chain-certificate pem file
        #[arg(short = 'c', long = "intermediate-full-chain-cert")]
        intermediate_full_chain_cert: std::path::PathBuf,
        /// path to intermediate key pem file
        #[arg(short = 'k', long = "intermediate-key")]
        intermediate_key: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// device id
        #[arg(short = 'd', long = "device-id")]
        device_id: std::string::String,
        /// period of validity in days
        #[arg(short = 'D', long = "days")]
        days: u32,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(Parser, Debug)]
#[command(after_help="Copyright © 2021 by conplement AG")]
/// pre-configure wifi settings
pub enum WifiConfig {
    /// set wpa_supplicant.conf to pre-configure wifi settings
    Set {
        /// path to config file
        #[arg(short = 'c', long = "config")]
        config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(Parser, Debug)]
#[command(after_help="Copyright © 2021 by conplement AG")]
/// pre-configure enrollment settings
pub enum EnrollmentConfig {
    /// set enrollment configuration for images built with enrollment feature
    Set {
        /// path to enrollment config file
        #[arg(short = 'c', long = "enrollment-config")]
        enrollment_config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(Parser, Debug)]
#[command(after_help="Copyright © 2021 by conplement AG")]
/// pre-configure ADU settings
pub enum IotHubDeviceUpdateConfig {
    /// set ADU configuration
    Set {
        /// path to ADU config file
        #[arg(short = 'c', long = "config")]
        iot_hub_device_update_config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(Parser, Debug)]
#[command(after_help="Copyright © 2021 by conplement AG")]
/// pre-configure boot settings
pub enum BootConfig {
    Set {
        /// path to boot.scr file
        #[arg(short = 'c', long = "config")]
        boot_script: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(Parser, Debug)]
#[command(version, after_help="Copyright © 2021 by conplement AG")]
/// This tools helps to manage your ics-dm devices. For more information visit:\nhttps://github.com/ICS-DeviceManagement/ics-dm-cli
pub enum Command {
    #[command(subcommand)]
    Identity(IdentityConfig),
    DockerInfo,
    #[command(subcommand)]
    Wifi(WifiConfig),
    #[command(subcommand)]
    Enrollment(EnrollmentConfig),
    #[command(subcommand)]
    IotHubDeviceUpdate(IotHubDeviceUpdateConfig),
    #[command(subcommand)]
    Boot(BootConfig),
}

pub fn from_args() -> Command {
    Command::parse()
}
