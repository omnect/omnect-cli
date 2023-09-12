use clap::Parser;

const COPYRIGHT: &str = "Copyright © 2021 by conplement AG";

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// file handling
pub enum FileConfig {
    /// copy file into image
    Copy {
        /// path to input file
        #[arg(short = 'f', long = "file")]
        file: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// destination partition
        #[arg(short = 'p', long = "partition", value_enum)]
        partition: Partition,
        /// destination path
        #[arg(short = 'd', long = "destination")]
        destination: std::string::String,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(clap::ValueEnum, Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum Partition {
    boot,
    cert,
    factory,
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
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
        /// optional: path to payload file
        #[arg(short = 'p', long = "payload")]
        payload: Option<std::path::PathBuf>,
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
    /// set ssh tunnel certificate
    SetSshTunnelCertificate {
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// path to public key of the ssh root ca
        #[arg(short = 'r', long = "root_ca")]
        root_ca: std::path::PathBuf,
        /// device-id
        #[arg(short = 'p', long = "device-principal")]
        device_principal: String,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
    },
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
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
#[command(after_help = COPYRIGHT)]
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
#[command(after_help = COPYRIGHT)]
/// establish ssh connections to devices
pub struct SshConfig {
    /// username for the login on the device.
    #[arg(short = 'u', long = "user", default_value = "omnect")]
    pub username: String,
    /// optional: path where the ssh key pair, the certificates, and the
    /// temporary ssh configuration is stored. Defaults to system local data
    /// directories (e.g. ${XDG_RUNTIME_DIR}/omnect-cli on Linux).
    #[arg(short = 'd', long = "dir")]
    pub dir: Option<std::path::PathBuf>,
    /// optional: path to a pre-existing ssh private key that is used. Note:
    /// this expects the existence of a corresponding <key-path>.pub file.
    /// If not specified, omnect-cli creates a key pair for this connection.
    #[arg(short = 'k', long = "key")]
    pub priv_key_path: Option<std::path::PathBuf>,
    /// optional: path where the ssh configuration is stored. Defaults to system
    /// local data directories (e.g. ${XDG_RUNTIME_DIR}/omnect-cli/ssh_config on
    /// Linux).
    #[arg(short = 'c', long = "config-path")]
    pub config_path: Option<std::path::PathBuf>,
    /// address of the backend API
    #[arg(short = 'b', long = "backend")]
    pub backend: String,
    /// name of the device for which the ssh tunnel should be created.
    pub device: String,
}

#[derive(Parser, Debug)]
#[command(version, after_help = COPYRIGHT, verbatim_doc_comment)]
/// This tools helps to manage your omnect devices. For more information visit:
/// https://github.com/omnect/omnect-cli
pub enum Command {
    #[command(subcommand)]
    File(FileConfig),
    #[command(subcommand)]
    Identity(IdentityConfig),
    DockerInfo,
    #[command(subcommand)]
    Wifi(WifiConfig),
    #[command(subcommand)]
    IotHubDeviceUpdate(IotHubDeviceUpdateConfig),
    Ssh(SshConfig),
}

pub fn from_args() -> Command {
    Command::parse()
}
