use env_logger::{Builder, Env};
use log::error;
use std::process;

fn main() {
    // storage_account_client logs cleartext credentials, the others are just unnecessarily verbose.
    if cfg!(debug_assertions) {
        Builder::from_env(Env::default().default_filter_or(concat!(
            "debug",
            ",azure_core::http_client::reqwest=debug",
            ",azure_core::policies::transport=debug",
            ",azure_iot_deviceupdate::device_update=debug",
            ",azure_storage::core::clients::storage_account_client=info",
            ",azure_storage_blobs=info",
            ",device_update_importer::blob_uploader=info",
            ",reqwest::async_impl::client=debug"
        )))
        .init();
    } else {
        Builder::from_env(Env::default().default_filter_or(concat!(
            "info",
            ",azure_core::http_client::reqwest=debug",
            ",azure_core::policies::transport=debug",
            ",azure_iot_deviceupdate::device_update=debug",
            ",azure_storage::core::clients::storage_account_client=info",
            ",azure_storage_blobs=info",
            ",device_update_importer::blob_uploader=info",
            ",reqwest::async_impl::client=debug"
        )))
        .init();
    }

    if let Err(e) = omnect_cli::run() {
        error!("Application error: {e:#?}");

        process::exit(1);
    }
}
