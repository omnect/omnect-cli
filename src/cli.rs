use crate::file::{
    compression::Compression,
    functions::{FileCopyFromParams, FileCopyToParams},
};
use clap::Parser;

const COPYRIGHT: &str = "Copyright © 2021 by conplement AG";

// ToDo: command completion

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// file handling
pub enum FileConfig {
    /// copy file into image
    CopyToImage {
        /// multiple [in-file-path,out-partition:out-file-path]: input file, output file partition and output file
        #[clap(short = 'f', long = "files", value_parser = clap::value_parser!(FileCopyToParams), required(true))]
        file_copy_params: Vec<FileCopyToParams>,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
    /// copy file from image
    CopyFromImage {
        /// multiple [in-partition:in-file-path,out-file-path]: input file partition, input file and output file
        #[clap(short = 'f', long = "files", value_parser = clap::value_parser!(FileCopyFromParams), required(true))]
        file_copy_params: Vec<FileCopyFromParams>,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
    },
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
        /// optional: path to extra DPS payload file
        #[arg(short = 'e', long = "extra-dps-payload")]
        payload: Option<std::path::PathBuf>,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
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
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
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
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
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
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// iothub-device-update commands
pub enum IotHubDeviceUpdate {
    /// copy device-update configuration to image 
    SetDeviceConfig {
        /// path to device-update configuration file
        #[arg(short = 'c', long = "config")]
        iot_hub_device_update_config: std::path::PathBuf,
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
    /// import update to azure iothub
    Import,
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// ssh tunnel configuration
pub enum SshConfig {
    /// set ssh tunnel certificate
    SetCertificate {
        /// path to wic image file
        #[arg(short = 'i', long = "image")]
        image: std::path::PathBuf,
        /// path to public key of the ssh root ca
        #[arg(short = 'r', long = "root_ca")]
        root_ca: std::path::PathBuf,
        /// device-id
        #[arg(short = 'd', long = "device-principal")]
        device_principal: String,
        /// optional: generate bmap file
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },

    /// set ssh connection parameters
    SetConnection {
        /// username for the login on the device.
        #[arg(short = 'u', long = "user", default_value = "omnect")]
        username: String,
        /// optional: path where the ssh key pair, the certificates, and the
        /// temporary ssh configuration is stored. Defaults to system local data
        /// directories (e.g. ${XDG_RUNTIME_DIR}/omnect-cli on Linux).
        #[arg(short = 'd', long = "dir")]
        dir: Option<std::path::PathBuf>,
        /// optional: path to a pre-existing ssh private key that is used. Note:
        /// this expects the existence of a corresponding <key-path>.pub file.
        /// If not specified, omnect-cli creates a key pair for this connection.
        #[arg(short = 'k', long = "key")]
        priv_key_path: Option<std::path::PathBuf>,
        /// optional: path where the ssh configuration is stored. Defaults to system
        /// local data directories (e.g. ${XDG_RUNTIME_DIR}/omnect-cli/ssh_config on
        /// Linux).
        #[arg(short = 'c', long = "config-path")]
        config_path: Option<std::path::PathBuf>,
        /// path to a .toml configuration specifying the devices execution
        /// environment, defaults to the production environment.
        #[arg(short = 'e', long = "env")]
        env: Option<std::path::PathBuf>,
        /// name of the device for which the ssh tunnel should be created.
        device: String,
    },
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
    #[command(subcommand)]
    IotHubDeviceUpdate(IotHubDeviceUpdate),
    #[command(subcommand)]
    Ssh(SshConfig),
}

pub fn from_args() -> Command {
    Command::parse()
}
