use anyhow::Result;
use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use log::debug;

#[derive(Clone)]
pub struct BlobUploader {
    account: String,
    key: String,
    container: String,
}

impl BlobUploader {
    pub fn new(account: String, key: String, container: String) -> Self {
        BlobUploader {
            account,
            key,
            container,
        }
    }

    pub async fn write_blob(&self, name: &str, data: Vec<u8>) -> Result<url::Url> {
        let storage_credentials = StorageCredentials::access_key(&self.account, &self.key);

        let storage_client =
            ClientBuilder::new(&self.account, storage_credentials).blob_service_client();

        let container_client = storage_client.container_client(&self.container);

        anyhow::ensure!(
            container_client.exists().await?,
            "Container {} does not exist.",
            container_client.container_name()
        );

        let blob_client = container_client.blob_client(name);

        anyhow::ensure!(
            !(blob_client.exists().await?),
            "Blob {} already exists, will not overwrite.",
            name
        );

        let hash = md5::compute(&data).0;

        let put_block_response = blob_client.put_block_blob(data).hash(hash).await?;

        debug!("put_block_response {:#?}", put_block_response);

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

    pub async fn generate_sas_url(&self, name: &str) -> Result<url::Url> {
        let storage_credentials = StorageCredentials::access_key(&self.account, &self.key);
        let storage_account_client = BlobServiceClient::new(&self.account, storage_credentials);
        let blob_client = storage_account_client
            .container_client(&self.container)
            .blob_client(name);

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
}
