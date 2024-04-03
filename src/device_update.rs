use anyhow::{Context, Result};
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_iot_deviceupdate::DeviceUpdateClient;
use azure_storage::{shared_access_signature::service_sas::BlobSasPermissions, StorageCredentials};
use azure_storage_blobs::prelude::{BlobServiceClient, ContainerClient};
use log::{debug, info};
use serde::Serialize;
use sha2::Digest;
use std::{borrow::Cow, collections::HashMap, fs::OpenOptions, path::Path};
use time::format_description::well_known::Rfc3339;
use url::Url;

// See https://docs.microsoft.com/en-us/azure/iot-hub-device-update/device-update-limits
const MAX_DEVICE_UPDATE_SIZE: u64 = 2000000000; // 2GB, may also actually be 2^32 - 1?
const MANIFEST_VERSION: &str = "5.0";

#[derive(Serialize)]
struct UpdateId<'a> {
    provider: &'a str,
    name: &'a str,
    version: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserConsentHandlerProperties<'a> {
    installed_criteria: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SWUpdateHandlerProperties<'a> {
    installed_criteria: &'a str,
    swu_file_name: &'a str,
    arguments: &'a str,
    script_file_name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum HandlerProperties<'a> {
    UserConsent(UserConsentHandlerProperties<'a>),
    SWUpdate(SWUpdateHandlerProperties<'a>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Step<'a> {
    #[serde(rename = "type")]
    step_type: &'a str,
    description: &'a str,
    handler: &'a str,
    files: Vec<&'a str>,
    handler_properties: HandlerProperties<'a>,
}

#[derive(Serialize)]
struct Instructions<'a> {
    steps: Vec<Step<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct File<'a> {
    filename: Cow<'a, str>,
    size_in_bytes: u64,
    hashes: HashMap<&'a str, String>,
}

#[derive(Serialize)]
struct Compatibility<'a> {
    manufacturer: &'a str,
    model: &'a str,
    compatibilityid: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportManifest<'a> {
    update_id: UpdateId<'a>,
    is_deployable: bool,
    compatibility: Vec<Compatibility<'a>>,
    instructions: Instructions<'a>,
    files: Vec<&'a File<'a>>,
    created_date_time: &'a str,
    manifest_version: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileUrl<'a> {
    url: Url,
    size_in_bytes: u64,
    hashes: HashMap<&'a str, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileNameUrl<'a> {
    filename: &'a str,
    url: Url,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportUpdate<'a> {
    import_manifest: FileUrl<'a>,
    files: Vec<FileNameUrl<'a>>,
}

#[tokio::main]
#[allow(clippy::too_many_arguments)]
pub async fn create_import_manifest(
    image_path: &Path,
    script_path: &Path,
    manufacturer: &str,
    model: &str,
    compatibilityid: &str,
    provider: &str,
    consent_handler: &str,
    swupdate_handler: &str,
    name: &str,
    version: &str,
) -> Result<()> {
    let installed_criteria = format!("{name} {version}");
    let installed_criteria = installed_criteria.as_str();
    let image_attributes = get_file_attributes(image_path)?;
    let script_attributes = get_file_attributes(script_path)?;
    let import_manifest_path = format!("{}.importManifest.json", image_attributes.filename);
    let time_stamp = time::OffsetDateTime::now_utc().format(&Rfc3339)?;
    let steps = Vec::<Step>::from([
        Step {
            step_type: "inline",
            description: "User consent for swupdate",
            handler: consent_handler,
            files: vec![&image_attributes.filename],
            handler_properties: HandlerProperties::UserConsent(UserConsentHandlerProperties {
                installed_criteria,
            }),
        },
        Step {
            step_type: "inline",
            description: "Update rootfs using A/B update strategy",
            handler: swupdate_handler,
            files: vec![&image_attributes.filename, &script_attributes.filename],
            handler_properties: HandlerProperties::SWUpdate(SWUpdateHandlerProperties {
                swu_file_name: &image_attributes.filename,
                arguments: "",
                script_file_name: &script_attributes.filename,
                installed_criteria,
            }),
        },
    ]);

    let import_manifest = ImportManifest {
        update_id: UpdateId {
            provider,
            name,
            version,
        },
        is_deployable: true,
        compatibility: vec![Compatibility {
            manufacturer,
            model,
            compatibilityid,
        }],
        instructions: Instructions { steps },
        files: vec![&image_attributes, &script_attributes],
        created_date_time: time_stamp.as_str(),
        manifest_version: MANIFEST_VERSION,
    };

    serde_json::to_writer_pretty(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(import_manifest_path)
            .context("create import manifest file")?,
        &import_manifest,
    )
    .context("write import manifest file")?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tokio::main]
pub async fn import_update(
    import_manifest_path: &Path,
    container_name: &str,
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    instance_id: &str,
    device_update_endpoint_url: &Url,
    blob_storage_account: &str,
    blob_storage_key: &str,
) -> Result<()> {
    let creds = std::sync::Arc::new(ClientSecretCredential::new(
        azure_core::new_http_client(),
        tenant_id.to_string(),
        client_id.to_string(),
        client_secret.to_string(),
        TokenCredentialOptions::default(),
    ));
    let client = DeviceUpdateClient::new(device_update_endpoint_url.as_str(), creds)?;
    let manifest_file_size = std::fs::metadata(import_manifest_path)
        .context(format!(
            "cannot get file metadata of {}",
            import_manifest_path
                .to_str()
                .context("import manifest pah invalid")?
        ))?
        .len();
    let manifest_sha256 = base64::encode_config(
        sha2::Sha256::digest(std::fs::read(import_manifest_path).unwrap()),
        base64::STANDARD,
    );

    let manifest: serde_json::Value = serde_json::from_reader(
        OpenOptions::new()
            .read(true)
            .create(false)
            .open(import_manifest_path)
            .context("open import manifest file")?,
    )
    .context("read import manifest file")?;

    let file_name1 = manifest["instructions"]["steps"][1]["files"][0]
        .as_str()
        .context("step1 file not found")?
        .to_string();
    let file_name2 = manifest["instructions"]["steps"][1]["files"][1]
        .as_str()
        .context("step2 file not found")?
        .to_string();

    let storage_credentials =
        StorageCredentials::access_key(blob_storage_account, blob_storage_key);
    let storage_account_client = BlobServiceClient::new(blob_storage_account, storage_credentials);
    let container_client = storage_account_client.container_client(container_name);
    let import_manifest_path = import_manifest_path.file_name().unwrap().to_str().unwrap();
    let manifest_url = generate_sas_url(&container_client, import_manifest_path).await?;
    let file_url1 = generate_sas_url(&container_client, file_name1.clone()).await?;
    let file_url2 = generate_sas_url(&container_client, file_name2.clone()).await?;
    let import_update = vec![ImportUpdate {
        import_manifest: FileUrl {
            url: manifest_url,
            size_in_bytes: manifest_file_size,
            hashes: HashMap::from([("sha256", manifest_sha256)]),
        },
        files: vec![
            FileNameUrl {
                filename: &file_name1,
                url: file_url1,
            },
            FileNameUrl {
                filename: &file_name2,
                url: file_url2,
            },
        ],
    }];

    let import_update =
        serde_json::to_string_pretty(&import_update).context("Cannot parse import_update")?;

    debug!("import update: {import_update}");

    let import_update_response = client.import_update(instance_id, import_update).await?;
    info!("Result of import update: {:?}", &import_update_response);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tokio::main]
pub async fn remove_update(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    instance_id: &str,
    device_update_endpoint_url: &Url,
    provider: &str,
    name: &str,
    version: &str,
) -> Result<()> {
    let creds = std::sync::Arc::new(ClientSecretCredential::new(
        azure_core::new_http_client(),
        tenant_id.to_string(),
        client_id.to_string(),
        client_secret.to_string(),
        TokenCredentialOptions::default(),
    ));
    let client = DeviceUpdateClient::new(device_update_endpoint_url.as_str(), creds)?;

    debug!("remove update");

    let remove_update_response = client
        .delete_update(instance_id, provider, name, version)
        .await?;
    info!("Result of remove update: {remove_update_response}");

    Ok(())
}

fn get_file_attributes(file: &Path) -> Result<File> {
    debug!("get file attributes for {file:#?}");

    let filename = file.file_name().unwrap().to_string_lossy();

    let file = file.to_str().unwrap();

    let size_in_bytes = std::fs::metadata(file)
        .context(format!("cannot get file metadata of {}", file))?
        .len();

    anyhow::ensure!(
        size_in_bytes <= MAX_DEVICE_UPDATE_SIZE,
        "Azure device update limits the update file size to {}.",
        MAX_DEVICE_UPDATE_SIZE
    );

    let hashes = HashMap::from([(
        "sha256",
        base64::encode_config(
            sha2::Sha256::digest(std::fs::read(file).context(format!("cannot read {}", file))?),
            base64::STANDARD,
        ),
    )]);

    Ok(File {
        filename,
        size_in_bytes,
        hashes,
    })
}

pub async fn generate_sas_url(
    container_client: &ContainerClient,
    blob_name: impl Into<String>,
) -> Result<url::Url> {
    let blob_client = container_client.blob_client(blob_name);

    let token = blob_client
        .shared_access_signature(
            BlobSasPermissions {
                read: true,
                ..BlobSasPermissions::default()
            },
            time::OffsetDateTime::now_utc() + time::Duration::hours(12),
        )
        .await?;

    blob_client
        .generate_signed_blob_url(&token)
        .map_err(|e| e.into())
}
