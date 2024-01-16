mod blob_uploader;
use anyhow::{Context, Result};
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_iot_deviceupdate::DeviceUpdateClient;
use blob_uploader::BlobUploader;
use log::{debug, info};
use serde::Serialize;
use serde_json::json;
use sha2::Digest;
use std::collections::HashMap;
use std::env;
use time::format_description::well_known::Rfc3339;

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
    arguments: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    step_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
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
    hashes: HashMap<String, String>,
}

#[derive(Serialize)]
struct Compatibility {
    manufacturer: String,
    model: String,
    compatibilityid: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Update {
    update_id: UpdateId,
    is_deployable: bool,
    compatibility: Vec<Compatibility>,
    instructions: Instructions,
    files: Vec<File>,
    created_date_time: String,
    manifest_version: String,
}

struct FileAttributes {
    url: url::Url,
    basename: String,
    size: u64,
    sha256: String,
}

fn get_env(key: &str) -> Result<String> {
    env::var(key).context(format!("Cannot get envvar: {key}"))
}

#[tokio::main]
pub async fn import() -> Result<()> {
    let tenant_id = get_env("AZURE_TENANT_ID")?;
    let client_id = get_env("AZURE_CLIENT_ID")?;
    let client_secret = get_env("AZURE_CLIENT_SECRET")?;

    // credentials for deviceupdate
    let creds = std::sync::Arc::new(ClientSecretCredential::new(
        azure_core::new_http_client(),
        tenant_id,
        client_id,
        client_secret,
        TokenCredentialOptions::default(),
    ));
    let device_update_endpoint = get_env("AZURE_ACCOUNT_ENDPOINT")?;
    let client = DeviceUpdateClient::new(&device_update_endpoint, creds)?;

    let storage_name = get_env("AZURE_STORAGE_NAME")?;
    let storage_key = get_env("AZURE_STORAGE_KEY")?;
    let blob_container = get_env("AZURE_BLOB_CONTAINER")?;

    let instance_id = get_env("AZURE_INSTANCE_ID")?;

    let dev_prop_manufacturer = get_env("ADU_DEVICEPROPERTIES_MANUFACTURER")?;
    let dev_prop_model = get_env("ADU_DEVICEPROPERTIES_MODEL")?;
    let dev_prop_compatibilityid = get_env("ADU_DEVICEPROPERTIES_COMPATIBILITY_ID")?;
    let provider = get_env("ADU_PROVIDER")?;
    let update_type = get_env("ADU_UPDATE_TYPE")?;
    let consent_type = match get_env("ADU_CONSENT").unwrap_or_default().as_str() {
        "true" | "TRUE" | "1" | "required" => Some(get_env("ADU_CONSENT_TYPE")?),
        _ => None,
    };
    let image_name = get_env("OMNECT_IMAGE_NAME")?;
    let image_version = get_env("OMNECT_IMAGE_VERSION")?;
    let image_criteria = get_env("OMNECT_IMAGE_INSTALLED_CRITERIA")?;

    let uploader = BlobUploader::new(storage_name, storage_key, blob_container);

    let image_attributes = get_file_attributes(
        "OMNECT_IMAGE_PATH",
        "UPDATE_FILE_URI",
        "UPDATE_FILE_SIZE",
        "UPDATE_FILE_SHA256_BASE64",
        &uploader,
    )
    .await?;
    let script_attributes = get_file_attributes(
        "OMNECT_SCRIPT_PATH",
        "SCRIPT_FILE_URI",
        "SCRIPT_FILE_SIZE",
        "SCRIPT_FILE_SHA256_BASE64",
        &uploader,
    )
    .await?;

    let manifest_name = format!("{}.importManifest.json", image_attributes.basename);

    let mut steps = Vec::<Step>::new();
    if let Some(consent_type) = consent_type {
        steps.push(Step {
            step_type: Some("inline".to_string()),
            description: Some("User consent for swupdate".to_string()),
            handler: consent_type,
            files: vec![image_attributes.basename.clone()],
            handler_properties: HandlerProperties::UserConsent(UserConsentHandlerProperties {
                installed_criteria: image_criteria.clone(),
            }),
        });
    }
    steps.push(Step {
        step_type: Some("inline".to_string()),
        description: Some("Update rootfs using A/B update strategy".to_string()),
        handler: update_type,
        files: vec![
            image_attributes.basename.clone(),
            script_attributes.basename.clone(),
        ],
        handler_properties: HandlerProperties::SWUpdate(SWUpdateHandlerProperties {
            swu_file_name: image_attributes.basename.clone(),
            arguments: "".to_string(),
            script_file_name: script_attributes.basename.clone(),
            installed_criteria: image_criteria,
        }),
    });

    let current_time = time::OffsetDateTime::now_utc();
    let current_time = current_time.format(&Rfc3339)?;
    let update = Update {
        update_id: UpdateId {
            provider,
            name: image_name,
            version: image_version,
        },
        is_deployable: true,
        compatibility: vec![Compatibility {
            manufacturer: dev_prop_manufacturer,
            model: dev_prop_model,
            compatibilityid: dev_prop_compatibilityid,
        }],
        instructions: Instructions { steps },
        files: vec![
            File {
                filename: image_attributes.basename.clone(),
                size_in_bytes: image_attributes.size,
                hashes: HashMap::from([("sha256".to_string(), image_attributes.sha256)]),
            },
            File {
                filename: script_attributes.basename.clone(),
                size_in_bytes: script_attributes.size,
                hashes: HashMap::from([("sha256".to_string(), script_attributes.sha256)]),
            },
        ],
        created_date_time: current_time,
        manifest_version: MANIFEST_VERSION.to_string(),
    };
    let manifest_data = serde_json::to_vec_pretty(&update)?;
    let manifest_file_size = manifest_data.len();
    let manifest_sha256 =
        base64::encode_config(sha2::Sha256::digest(&manifest_data), base64::STANDARD);

    let manifest_url =  uploader.write_blob(&manifest_name, manifest_data).await.context("Error uploading import manifest")?;

    info!("Manifest is at {:?}", manifest_url);

    let import_manifest = json!(
    [
        {
            "importManifest": {
            "url": manifest_url,
            "sizeInBytes": manifest_file_size,
            "hashes": {
                "sha256": manifest_sha256
            }
            },
            "files": [
            {
                "filename": image_attributes.basename,
                "url": image_attributes.url
            },
            {
                "filename": script_attributes.basename,
                "url": script_attributes.url
            }
            ]
        }
    ]);
    let import_update_response = client
        .import_update(&instance_id, import_manifest.to_string())
        .await?;
    info!("Result of import: {:?}", &import_update_response);

    Ok(())
}

async fn get_file_attributes(
    file_path_env: &str,
    file_uri_env: &str,
    file_size_env: &str,
    file_sha256_env: &str,
    uploader: &BlobUploader,
) -> Result<FileAttributes> {
    let mut size = get_env(file_size_env)?.parse::<u64>()?;
    let mut sha256 = get_env(file_sha256_env)?;

    let r = match get_env(file_path_env) {
        Err(_e) => {
            debug!("{file_path_env} not set, assuming file already uploaded.");
            let basename = match get_env(file_uri_env) {
                Err(_e) => 
                    anyhow::bail!("Neither {file_path_env} nor {file_uri_env} set, no idea how to generate update from. Bailing out."),
                
                Ok(image_url_str) => {
                    debug!("{file_uri_env}: {image_url_str}");
                    let url = url::Url::parse(&image_url_str)?;

                    anyhow::ensure!(
                        size <= MAX_DEVICE_UPDATE_SIZE,
                        "Azure device update limits the update file size to {}.",
                        MAX_DEVICE_UPDATE_SIZE
                    );

                    let file_name = url
                        .path_segments().context("Cannot split image url")?         
                        .last()
                        .context("Cannot get file from image url")?;
                    file_name.to_owned()
                }
            };
            let url = uploader
                .generate_sas_url(&basename)
                .await
                .context("Error generating SAS storage url")?;

            (url, basename, size, sha256)
        }

        Ok(file_path) => {
            debug!("{file_path_env}: {file_path}");

            anyhow::ensure!(std::fs::metadata(&file_path)?.len() <= MAX_DEVICE_UPDATE_SIZE,  "Azure device update limits the update file size to {}.",
            MAX_DEVICE_UPDATE_SIZE);

            let basename = std::path::Path::new(&file_path)
                .file_name()
                .context("Getting basename failed.")?
                .to_str()
                .context("Converting basename to utf8 failed")?
                .to_owned();
            let data = std::fs::read(file_path)?;
            size = data.len() as u64;

            sha256 = base64::encode_config(sha2::Sha256::digest(&data), base64::STANDARD);

            let url =  uploader.write_blob(&basename, data).await.context("Error uploading file")?;
            info!("File is at {url}");

            (url, basename, size, sha256)
        }
    };

    Ok(FileAttributes {
        url: r.0,
        basename: r.1,
        size: r.2,
        sha256: r.3,
    })
}
