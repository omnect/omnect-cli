use anyhow::{Context, Result};
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_iot_deviceupdate::DeviceUpdateClient;
use azure_storage::{shared_access_signature::service_sas::BlobSasPermissions, StorageCredentials};
use azure_storage_blobs::prelude::{BlobServiceClient, ContainerClient};
use log::{debug, info};
use serde::Serialize;
use serde_json::json;
use sha2::Digest;
use std::{collections::HashMap, fs::OpenOptions, path::Path};
use time::format_description::well_known::Rfc3339;
use url::Url;

// See https://docs.microsoft.com/en-us/azure/iot-hub-device-update/device-update-limits
const MAX_DEVICE_UPDATE_SIZE: u64 = 2000000000; // 2GB, may also actually be 2^32 - 1?
const MANIFEST_VERSION: &str = "5.0";

#[derive(Serialize)]
struct UpdateId {
    provider: String,
    name: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserConsentHandlerProperties {
    installed_criteria: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SWUpdateHandlerProperties {
    installed_criteria: String,
    swu_file_name: String,
    arguments: &'static str,
    script_file_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum HandlerProperties {
    UserConsent(UserConsentHandlerProperties),
    SWUpdate(SWUpdateHandlerProperties),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Step {
    #[serde(rename = "type")]
    step_type: &'static str,
    description: &'static str,
    handler: String,
    files: Vec<String>,
    handler_properties: HandlerProperties,
}

#[derive(Serialize)]
struct Instructions {
    steps: Vec<Step>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct File {
    filename: String,
    size_in_bytes: u64,
    hashes: HashMap<&'static str, String>,
}

#[derive(Serialize)]
struct Compatibility {
    manufacturer: String,
    model: String,
    compatibilityid: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportManifest {
    update_id: UpdateId,
    is_deployable: bool,
    compatibility: Vec<Compatibility>,
    instructions: Instructions,
    files: Vec<File>,
    created_date_time: String,
    manifest_version: &'static str,
}

struct FileAttributes {
    basename: String,
    size: u64,
    sha256: String,
}

#[tokio::main]
#[allow(clippy::too_many_arguments)]
pub async fn create_import_manifest(
    image_path: &Path,
    script_path: &Path,
    manufacturer: String,
    model: String,
    compatibilityid: String,
    provider: String,
    consent_handler: String,
    swupdate_handler: String,
    name: String,
    version: String,
) -> Result<()> {
    let installed_criteria = format!("{name} {version}");
    let image_attributes = get_file_attributes(image_path)?;
    let script_attributes = get_file_attributes(script_path)?;
    let steps = Vec::<Step>::from([
        Step {
            step_type: "inline",
            description: "User consent for swupdate",
            handler: consent_handler,
            files: vec![image_attributes.basename.clone()],
            handler_properties: HandlerProperties::UserConsent(UserConsentHandlerProperties {
                installed_criteria: installed_criteria.clone(),
            }),
        },
        Step {
            step_type: "inline",
            description: "Update rootfs using A/B update strategy",
            handler: swupdate_handler,
            files: vec![
                image_attributes.basename.clone(),
                script_attributes.basename.clone(),
            ],
            handler_properties: HandlerProperties::SWUpdate(SWUpdateHandlerProperties {
                swu_file_name: image_attributes.basename.clone(),
                arguments: "",
                script_file_name: script_attributes.basename.clone(),
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
        files: vec![
            File {
                filename: image_attributes.basename.clone(),
                size_in_bytes: image_attributes.size,
                hashes: HashMap::from([("sha256", image_attributes.sha256)]),
            },
            File {
                filename: script_attributes.basename.clone(),
                size_in_bytes: script_attributes.size,
                hashes: HashMap::from([("sha256", script_attributes.sha256)]),
            },
        ],
        created_date_time: time::OffsetDateTime::now_utc().format(&Rfc3339)?,
        manifest_version: MANIFEST_VERSION,
    };

    serde_json::to_writer_pretty(
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("{}.importManifest.json", image_attributes.basename))
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
    container_name: String,
    tenant_id: String,
    client_id: String,
    client_secret: String,
    instance_id: String,
    device_update_endpoint_url: &Url,
    blob_storage_account: String,
    blob_storage_key: String,
) -> Result<()> {
    // credentials for deviceupdate
    let creds = std::sync::Arc::new(ClientSecretCredential::new(
        azure_core::new_http_client(),
        tenant_id,
        client_id,
        client_secret,
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
        .context("step1 file not found")?;
    let file_name2 = manifest["instructions"]["steps"][1]["files"][1]
        .as_str()
        .context("step2 file not found")?;

    let storage_credentials =
        StorageCredentials::access_key(&blob_storage_account, &blob_storage_key);
    let storage_account_client = BlobServiceClient::new(&blob_storage_account, storage_credentials);
    let container_client = storage_account_client.container_client(&container_name);
    let import_manifest_path = import_manifest_path.file_name().unwrap().to_str().unwrap();
    let manifest_url = generate_sas_url(&container_client, import_manifest_path).await?;
    let file_url1 = generate_sas_url(&container_client, file_name1).await?;
    let file_url2 = generate_sas_url(&container_client, file_name2).await?;
    let import_manifest = json!(
    [
        {
            "importManifest":
            {
                "url": manifest_url,
                "sizeInBytes": manifest_file_size,
                "hashes": {
                    "sha256": manifest_sha256
                }
            },
            "files": [
                {
                    "filename": file_name1,
                    "url": file_url1
                },
                {
                    "filename": file_name2,
                    "url": file_url2
                }
            ]
        }
    ]);

    debug!("{}", import_manifest.to_string());

    let import_update_response = client
        .import_update(&instance_id, import_manifest.to_string())
        .await?;
    info!("Result of import: {:?}", &import_update_response);

    Ok(())
}

fn get_file_attributes(file: &Path) -> Result<FileAttributes> {
    debug!("get file attributes for {file:#?}");

    let basename = file
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();

    let file = file.to_str().unwrap();

    let size = std::fs::metadata(file)
        .context(format!("cannot get file metadata of {}", file))?
        .len();

    anyhow::ensure!(
        size <= MAX_DEVICE_UPDATE_SIZE,
        "Azure device update limits the update file size to {}.",
        MAX_DEVICE_UPDATE_SIZE
    );

    let sha256 = base64::encode_config(
        sha2::Sha256::digest(std::fs::read(file).context(format!("cannot read {}", file))?),
        base64::STANDARD,
    );

    Ok(FileAttributes {
        basename,
        size,
        sha256,
    })
}

pub async fn generate_sas_url(
    container_client: &ContainerClient,
    blob_name: &str,
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
