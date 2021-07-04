use ics_dm_cli::wifi;
use ics_dm_cli::identity;
use std::path::PathBuf;

mod common;


#[test]
fn check_set_wificonfig() {
    common::setup();

    let config_file_path = PathBuf::from(r"tests/testfiles/wpa_supplicant.conf");
    let image_path = PathBuf::from(r"tests/testfiles/image.wic");

    assert_eq!(true, wifi::config(config_file_path, image_path).is_ok());

    common::cleanup();
}

#[test]
fn check_set_identity() {
    common::setup();

    common::cleanup();
}