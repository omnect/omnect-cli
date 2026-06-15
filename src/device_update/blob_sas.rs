use anyhow::{Context, Result};
use base64::prelude::*;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use time::OffsetDateTime;
use url::Url;
use url::form_urlencoded;

const SAS_VERSION: &str = "2022-11-02";

/// Generates a Service SAS URL for a blob with the given permissions and expiry.
///
/// Uses HMAC-SHA256 signing per the Azure Storage Service SAS specification.
/// Reference: https://learn.microsoft.com/en-us/rest/api/storageservices/create-service-sas
pub(crate) fn generate_blob_sas_url(
    account: &str,
    account_key: &str,
    container: &str,
    blob: &str,
    permissions: &str,
    expiry: OffsetDateTime,
) -> Result<Url> {
    let expiry_str = format_sas_time(expiry);
    let canonicalized_resource = format!("/blob/{account}/{container}/{blob}");

    // StringToSign for sv=2022-11-02 (16 fields, 15 newlines):
    //   signedPermissions, signedStart, signedExpiry, canonicalizedResource,
    //   signedIdentifier, signedIP, signedProtocol, signedVersion,
    //   signedResource, signedSnapshotTime, signedEncryptionScope,
    //   rscc, rscd, rsce, rscl, rsct
    let string_to_sign = format!(
        "{permissions}\n\
         \n\
         {expiry_str}\n\
         {canonicalized_resource}\n\
         \n\
         \n\
         https\n\
         {SAS_VERSION}\n\
         b\n\
         \n\
         \n\
         \n\
         \n\
         \n\
         \n"
    );

    let signature = sign_hmac_sha256(account_key, &string_to_sign)?;

    let sas_query: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("sp", permissions)
        .append_pair("se", &expiry_str)
        .append_pair("spr", "https")
        .append_pair("sv", SAS_VERSION)
        .append_pair("sr", "b")
        .append_pair("sig", &signature)
        .finish();

    let url_str = format!("https://{account}.blob.core.windows.net/{container}/{blob}?{sas_query}");

    Url::parse(&url_str).context("failed to construct SAS URL")
}

fn sign_hmac_sha256(account_key_base64: &str, message: &str) -> Result<String> {
    let key_bytes = BASE64_STANDARD
        .decode(account_key_base64)
        .context("invalid base64 storage account key")?;

    let mut mac = Hmac::<Sha256>::new_from_slice(&key_bytes).context("invalid HMAC key length")?;

    mac.update(message.as_bytes());

    Ok(BASE64_STANDARD.encode(mac.finalize().into_bytes()))
}

fn format_sas_time(dt: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_sas_time_produces_iso8601_utc() {
        let dt = OffsetDateTime::from_unix_timestamp(1684893235).expect("valid timestamp");
        assert_eq!(format_sas_time(dt), "2023-05-24T01:53:55Z");
    }

    #[test]
    fn sign_hmac_sha256_produces_deterministic_output() {
        // base64("testkey1") = "dGVzdGtleTEK" — but we need a real base64 key
        let key = BASE64_STANDARD.encode(b"test-storage-key");
        let sig1 = sign_hmac_sha256(&key, "test message").expect("sign works");
        let sig2 = sign_hmac_sha256(&key, "test message").expect("sign works");
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn generate_blob_sas_url_produces_valid_url() {
        let key = BASE64_STANDARD.encode(b"fake-storage-account-key-for-testing");
        let expiry = OffsetDateTime::from_unix_timestamp(1716544435).expect("valid timestamp");

        let url =
            generate_blob_sas_url("myaccount", &key, "mycontainer", "myblob.txt", "r", expiry)
                .expect("SAS URL generation works");

        assert!(
            url.as_str()
                .starts_with("https://myaccount.blob.core.windows.net/mycontainer/myblob.txt?")
        );
        assert!(url.as_str().contains("sp=r"));
        assert!(url.as_str().contains("sr=b"));
        assert!(url.as_str().contains("sv=2022-11-02"));
        assert!(url.as_str().contains("sig="));
        assert!(url.as_str().contains("spr=https"));
    }
}
