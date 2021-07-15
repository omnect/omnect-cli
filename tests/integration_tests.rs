use ics_dm_cli::docker;
use std::path::PathBuf;

mod common;


#[test]
fn check_set_wifi_config() {
    common::setup();

    let config_file_path = PathBuf::from(r"tests/testfiles/wpa_supplicant.conf");
    let image_path = PathBuf::from(r"tests/testfiles/image.wic");

    assert_eq!(true, docker::set_wifi_config(&config_file_path, &image_path).is_ok());

    common::cleanup();
}

#[test]
fn check_set_enrollment_config() {
    
    common::setup();

    let enrollment_config_file_path = PathBuf::from(r"tests/testfiles/enrollment_static.conf");
    let image_path = PathBuf::from(r"tests/testfiles/image.wic");

    assert_eq!(true, docker::set_enrollment_config(&enrollment_config_file_path, &image_path).is_ok());

    common::cleanup();
}

#[test]
fn check_set_identity_gateway_config() {
    common::setup();

    let config_file_path = PathBuf::from(r"tests/testfiles/config.toml");
    let image_path = PathBuf::from(r"tests/testfiles/image.wic");
    let root_ca_file_path = PathBuf::from(r"tests/testfiles/root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = PathBuf::from(r"tests/testfiles/full-chain.cert.pem");
    let edge_device_identity_key_file_path = PathBuf::from(r"tests/testfiles/device-ca.key.pem");

    assert_eq!(true, docker::set_iotedge_gateway_config(&config_file_path, &image_path, &root_ca_file_path, &edge_device_identity_full_chain_file_path, &edge_device_identity_key_file_path).is_ok());

    common::cleanup();
}

#[test]
fn check_set_identity_leaf_config() {
    common::setup();

    let config_file_path = PathBuf::from(r"tests/testfiles/config.toml");
    let image_path = PathBuf::from(r"tests/testfiles/image.wic");
    let root_ca_file_path = PathBuf::from(r"tests/testfiles/root.ca.cert.pem");
    
    assert_eq!(true, docker::set_iot_leaf_sas_config(&config_file_path, &image_path, &root_ca_file_path).is_ok());

    common::cleanup();
}

#[test]
fn check_set_identity_config() {
    common::setup();

    let config_file_path = PathBuf::from(r"tests/testfiles/config.toml");
    let image_path = PathBuf::from(r"tests/testfiles/image.wic");
    
    assert_eq!(true, docker::set_identity_config(&config_file_path, &image_path).is_ok());

    common::cleanup();
}