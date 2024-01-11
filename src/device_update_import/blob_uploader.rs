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

    pub async fn write_blob(
        &self,
        name: &str,
        data: Vec<u8>,
    ) -> Result<url::Url> {
        let storage_credentials =
            StorageCredentials::access_key(self.account.clone(), self.key.clone());

        let storage_client = BlobServiceClient::new(&self.account, storage_credentials);

        let container_client = storage_client.container_client(self.container.clone());

        let container_create = container_client
            .create()
            .public_access(PublicAccess::None)
            .into_future()
            .await;

        match container_create {
            Ok(_c) => {
                debug!("Container {} created successfully.", self.container);
            }
            Err(e) => {
                debug!("Could not create container: {e}, continuing...");
            }
        }

        let blob_client = storage_client
            .container_client(self.container.clone())
            .blob_client(name);

        if blob_client.exists().await? {
            log::warn!("Blob {} already exists, will not overwrite.", name);
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, name).into());
        }

        let block_id = bytes::Bytes::from(format!("{}", 1));
        let hash = md5::compute(data.clone()).0;

        let put_block_response = blob_client
            .put_block(block_id.clone(), data)
            .hash(hash)
            .into_future()
            .await?;

        debug!("put_block_response {:#?}", put_block_response);

        let mut block_list = BlockList::default();
        block_list
            .blocks
            .push(BlobBlockType::new_uncommitted(block_id));

        let content_hash = BlobContentMD5::from(hash);
        let res = blob_client
            .put_block_list(block_list)
            .content_md5(content_hash)
            .into_future()
            .await?;
        debug!("PutBlockList == {:?}", res);

        let token = blob_client
            .shared_access_signature(
                BlobSasPermissions {
                    read: true,
                    ..BlobSasPermissions::default()
                },
                time::OffsetDateTime::now_utc() + time::Duration::hours(12),
            )
            .await?;

        let url = blob_client.generate_signed_blob_url(&token)?;
        Ok(url)
    }

    pub async fn generate_sas_url(
        &self,
        name: &str,
    ) -> Result<url::Url> {
        let storage_credentials =
            StorageCredentials::access_key(self.account.clone(), self.key.clone());
        let storage_account_client = BlobServiceClient::new(&self.account, storage_credentials);
        let blob_client = storage_account_client
            .container_client(self.container.clone())
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
        let url = blob_client.generate_signed_blob_url(&token)?;
        Ok(url)
    }
}
