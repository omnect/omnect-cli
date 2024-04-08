use crate::file::{
    compression::Compression,
    functions::{FileCopyFromParams, FileCopyToParams},
};
use clap::Parser;
use std::path::PathBuf;
use url::Url;

const COPYRIGHT: &str = "Copyright © 2021 by conplement AG";

// ToDo: command completion

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// copy files to or from a firmware image
pub enum File {
    /// file commands, e.g. copy multiple files to/from image
    CopyToImage {
        /// vector of copy triples in the format [in-file-path,out-partition:out-file-path]
        #[clap(short = 'f', long = "files", value_parser = clap::value_parser!(FileCopyToParams), required(true))]
        file_copy_params: Vec<FileCopyToParams>,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// optional: generate bmap file (currently not working in docker image)
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
    /// copy files from image
    CopyFromImage {
        /// vector of copy triples in the format [in-partition:in-file-path,out-file-path]
        #[clap(short = 'f', long = "files", value_parser = clap::value_parser!(FileCopyFromParams), required(true))]
        file_copy_params: Vec<FileCopyFromParams>,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
    },
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// configure Azure IoT identity settings
pub enum IdentityConfig {
    /// configure identity settings of a standard iot or iotedge device (no transparent gateway nor iot leaf device)
    SetConfig {
        /// path to config.toml file
        #[arg(short = 'c', long = "config")]
        config: PathBuf,
        /// optional: path to extra DPS payload file
        #[arg(short = 'e', long = "extra-dps-payload")]
        payload: Option<PathBuf>,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// optional: generate bmap file (currently not working in docker image)
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
    /// EXPERIMENTAL: set transparent gateway config.toml file and additional certificates and keys
    SetIotedgeGatewayConfig {
        /// path to config.toml file
        #[arg(short = 'c', long = "config")]
        config: PathBuf,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// path to root ca certificate file
        #[arg(short = 'r', long = "root_ca")]
        root_ca: PathBuf,
        /// path to device identity certificate file
        #[arg(short = 'd', long = "device_identity")]
        device_identity: PathBuf,
        /// path to device identity certificate key file
        #[arg(short = 'k', long = "device_identity_key")]
        device_identity_key: PathBuf,
        /// optional: generate bmap file (currently not working in docker image)
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
    /// EXPERIMENTAL: set leaf device config.toml file and additional certificate
    SetIotLeafSasConfig {
        /// path to config.toml file
        #[arg(short = 'c', long = "config")]
        config: PathBuf,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// path to root ca certificate file
        #[arg(short = 'r', long = "root_ca")]
        root_ca: PathBuf,
        /// optional: generate bmap file (currently not working in docker image)
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
        intermediate_full_chain_cert: PathBuf,
        /// path to intermediate key pem file
        #[arg(short = 'k', long = "intermediate-key")]
        intermediate_key: PathBuf,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// device id
        #[arg(short = 'd', long = "device-id")]
        device_id: std::string::String,
        /// period of validity in days
        #[arg(short = 'D', long = "days")]
        days: u32,
        /// optional: generate bmap file (currently not working in docker image)
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// commands related to firmware updates via "Azure Device Update for IoT Hub"
pub enum IotHubDeviceUpdate {
    /// copy device update configuration to image
    SetDeviceConfig {
        /// path to device-update configuration file
        #[arg(short = 'c', long = "config")]
        iot_hub_device_update_config: PathBuf,
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// optional: generate bmap file (currently not working in docker image)
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },
    /// import update to azure iot-hub
    ImportUpdate {
        /// path to import manifest file
        #[arg(short = 'm', long = "import-manifest")]
        import_manifest: PathBuf,
        /// name of blob storage container where update image, script and import manifest files are located
        #[arg(short = 'n', long = "storage-container-name")]
        storage_container_name: String,
        /// azure tenant id
        #[arg(short = 't', long = "tenant-id")]
        tenant_id: String,
        /// azure client id
        #[arg(short = 'c', long = "client-id")]
        client_id: String,
        /// azure client secret
        #[arg(short = 's', long = "client-secret")]
        client_secret: String,
        /// azure instance id
        #[arg(short = 'i', long = "instance-id")]
        instance_id: String,
        /// url of iot-hub device update endpoint
        #[arg(short = 'e', long = "device-update-endpoint")]
        device_update_endpoint_url: Url,
        /// blob storage account name
        #[arg(short = 'a', long = "blob-storage-account")]
        blob_storage_account: String,
        /// blob storage key
        #[arg(short = 'k', long = "blob-storage-key")]
        blob_storage_key: String,
    },
    /// remove update from azure iot-hub
    RemoveUpdate {
        /// azure tenant id
        #[arg(short = 't', long = "tenant-id")]
        tenant_id: String,
        /// azure client id
        #[arg(short = 'c', long = "client-id")]
        client_id: String,
        /// azure client secret
        #[arg(short = 's', long = "client-secret")]
        client_secret: String,
        /// azure instance id
        #[arg(short = 'i', long = "instance-id")]
        instance_id: String,
        /// url of iot-hub device update endpoint
        #[arg(short = 'e', long = "device-update-endpoint")]
        device_update_endpoint_url: Url,
        /// overwrite default update provider
        #[arg(short = 'p', long = "provider", default_value = "conplement-AG")]
        provider: String,
        /// distro variant, e.g. OMNECT-gateway or OMNECT-gateway-devel
        #[arg(short = 'd', long = "distro-variant")]
        distro_name: String,
        /// image version
        #[arg(short = 'v', long = "version")]
        version: String,
    },
    /// create import manifest
    CreateImportManifest {
        /// distro variant, e.g. OMNECT-gateway or OMNECT-gateway-devel
        #[arg(short = 'd', long = "distro-variant")]
        distro_name: String,
        /// image version
        #[arg(short = 'v', long = "version")]
        version: String,
        /// path to swupdate image file
        #[arg(short = 'i', long = "swuimage")]
        image: PathBuf,
        /// path to update script file
        #[arg(short = 's', long = "script")]
        script: PathBuf,
        /// overwrite default update manufacturer
        #[arg(short = 'm', long = "manufacturer", default_value = "conplement-ag")]
        manufacturer: String,
        /// update model
        #[arg(short = 'n', long = "model")]
        model: String,
        /// update compatibility-id
        #[arg(short = 'c', long = "compatibilityid")]
        compatibilityid: String,
        /// overwrite default update provider
        #[arg(short = 'p', long = "provider", default_value = "conplement-AG")]
        provider: String,
        /// overwrite default consent handler
        #[arg(
            short = 'l',
            long = "consent-handler",
            default_value = "omnect/swupdate_consent:1"
        )]
        consent_handler: String,
        /// overwrite default swupdate handler
        #[arg(
            short = 'u',
            long = "swuupdate-handler",
            default_value = "microsoft/swupdate:2"
        )]
        swupdate_handler: String,
    },
}

#[derive(Parser, Debug)]
#[command(after_help = COPYRIGHT)]
/// ssh tunnel configuration
pub enum SshConfig {
    /// set ssh tunnel certificate
    SetCertificate {
        /// path to wic image file (optionally compressed with xz, bzip2 or gzip)
        #[arg(short = 'i', long = "image")]
        image: PathBuf,
        /// path to public key of the ssh root ca
        #[arg(short = 'r', long = "root_ca")]
        root_ca: PathBuf,
        /// optional: generate bmap file (currently not working in docker image)
        #[arg(short = 'b', long = "generate-bmap-file")]
        generate_bmap: bool,
        /// optional: pack image [xz, bzip2, gzip]
        #[arg(short = 'p', long = "pack-image", value_enum)]
        compress_image: Option<Compression>,
    },

    /// set ssh connection parameters (currently not working in docker image)
    SetConnection {
        /// username for the login on the device.
        #[arg(short = 'u', long = "user", default_value = "omnect")]
        username: String,
        /// optional: path where the ssh key pair, the certificates, and the
        /// temporary ssh configuration is stored. Defaults to system local data
        /// directories (e.g. ${XDG_RUNTIME_DIR}/omnect-cli on Linux).
        #[arg(short = 'd', long = "dir")]
        dir: Option<PathBuf>,
        /// optional: path to a pre-existing ssh private key that is used. Note:
        /// this expects the existence of a corresponding <key-path>.pub file.
        /// If not specified, omnect-cli creates a key pair for this connection.
        #[arg(short = 'k', long = "key")]
        priv_key_path: Option<PathBuf>,
        /// optional: path where the ssh configuration is stored. Defaults to system
        /// local data directories (e.g. ${XDG_RUNTIME_DIR}/omnect-cli/ssh_config on
        /// Linux).
        #[arg(short = 'c', long = "config-path")]
        config_path: Option<PathBuf>,
        /// optional: path to a .toml configuration specifying the devices execution
        /// environment, defaults to the production environment.
        #[arg(short = 'e', long = "env")]
        env: Option<PathBuf>,
        /// name of the device for which the ssh tunnel should be created.
        device: String,
    },
}

#[derive(Parser, Debug)]
#[command(version, after_help = COPYRIGHT, verbatim_doc_comment)]
/// This tool helps to manage your omnect devices. For more information visit:
/// https://github.com/omnect/omnect-cli
pub enum Command {
    #[command(subcommand)]
    File(File),
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
