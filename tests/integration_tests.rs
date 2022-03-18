mod common;
use common::Testrunner;
use crypto;
use ics_dm_cli::docker;
use stdext::function_name;
#[macro_use]
extern crate lazy_static;

#[test]
fn check_set_wifi_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("wpa_supplicant.conf");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(&config_file_path, &image_path, false).is_ok()
    );
}

#[test]
#[ignore]
fn check_set_wifi_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("wpa_supplicant.conf");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(&config_file_path, &image_path, true).is_ok()
    );
}

#[test]
fn check_set_enrollment_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, false).is_ok()
    );
}

#[test]
#[ignore]
fn check_set_enrollment_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, true).is_ok()
    );
}

#[test]
fn check_set_enrollment_config_missing_dps() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static_missing_dps.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_ne!(
        None,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, false)
            .unwrap_err()
            .to_string()
            .find("missing field `dpsConnectionString`")
    );
}

#[test]
fn check_set_enrollment_config_missing_iothub() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static_missing_iothub.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_ne!(
        None,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, false)
            .unwrap_err()
            .to_string()
            .find("missing field `iothubConnectionString`")
    );
}

#[test]
fn check_set_enrollment_config_unknown_key() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static_unknown_key.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, false).is_ok()
    );
}

#[test]
fn check_set_enrollment_config_invalid_connection_string() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path =
        tr.to_pathbuf("enrollment_static_invalid_connection_string.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_ne!(
        None,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, false)
            .unwrap_err()
            .to_string()
            .find("Enrollment validation failed")
    );
}

#[test]
fn check_set_identity_gateway_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("identity_config_gateway.toml");
    let image_path = tr.to_pathbuf("image.wic");
    let root_ca_file_path = tr.to_pathbuf("root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("device-ca.key.pem");

    assert_eq!(
        true,
        docker::set_iotedge_gateway_config(
            &config_file_path,
            &image_path,
            &root_ca_file_path,
            &edge_device_identity_full_chain_file_path,
            &edge_device_identity_key_file_path,
            false
        )
        .is_ok()
    );
}

#[test]
#[ignore]
fn check_set_identity_gateway_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("identity_config_gateway.toml");
    let image_path = tr.to_pathbuf("image.wic");
    let root_ca_file_path = tr.to_pathbuf("root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("device-ca.key.pem");

    assert_eq!(
        true,
        docker::set_iotedge_gateway_config(
            &config_file_path,
            &image_path,
            &root_ca_file_path,
            &edge_device_identity_full_chain_file_path,
            &edge_device_identity_key_file_path,
            true
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_leaf_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("identity_config_leaf.toml");
    let image_path = tr.to_pathbuf("image.wic");
    let root_ca_file_path = tr.to_pathbuf("root.ca.cert.pem");

    assert_eq!(
        true,
        docker::set_iot_leaf_sas_config(&config_file_path, &image_path, &root_ca_file_path, false)
            .is_ok()
    );
}

#[test]
#[ignore]
fn check_set_identity_leaf_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("identity_config_leaf.toml");
    let image_path = tr.to_pathbuf("image.wic");
    let root_ca_file_path = tr.to_pathbuf("root.ca.cert.pem");

    assert_eq!(
        true,
        docker::set_iot_leaf_sas_config(&config_file_path, &image_path, &root_ca_file_path, true)
            .is_ok()
    );
}

#[test]
fn check_set_identity_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("identity_config_dps_tpm.toml");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(&config_file_path, &image_path, false).is_ok()
    );
}

#[test]
#[ignore]
fn check_set_identity_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("identity_config_dps_tpm.toml");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(&config_file_path, &image_path, true).is_ok()
    );
}

#[test]
fn check_set_device_cert() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let intermediate_full_chain_crt_path = tr.to_pathbuf("intermediate_full_chain_cert.pem");
    let intermediate_full_chain_crt_key_path = tr.to_pathbuf("intermediate_cert_key.pem");

    let intermediate_full_chain_crt = std::fs::read_to_string(&intermediate_full_chain_crt_path)
        .expect("could not read intermediate full-chain-certificate");
    let intermediate_full_chain_crt_key =
        std::fs::read_to_string(&intermediate_full_chain_crt_key_path)
            .expect("could not read intermediate certificate key");

    let crypto = crypto::Crypto::new(
        intermediate_full_chain_crt_key.as_bytes(),
        intermediate_full_chain_crt.as_bytes(),
    )
    .expect("could not create crypto");

    let (device_cert_pem, device_key_pem) = crypto
        .new_device("bla")
        .expect("could not create new device certificate");

    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_device_cert(
            &intermediate_full_chain_crt_path,
            &device_cert_pem,
            &device_key_pem,
            &image_path,
            false
        )
        .is_ok()
    );
}

#[test]
fn check_set_iot_hub_device_update_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("du-config.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_iot_hub_device_update_config(&adu_config_file_path, &image_path, false).is_ok()
    );
}

#[test]
#[ignore]
fn check_set_iot_hub_device_update_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("du-config.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_iot_hub_device_update_config(&adu_config_file_path, &image_path, true).is_ok()
    );
}

#[test]
fn check_set_boot_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let boot_config_file_path = tr.to_pathbuf("boot.scr");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_boot_config(&boot_config_file_path, &image_path, false).is_ok()
    );
}

#[test]
#[ignore]
fn check_set_boot_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let boot_config_file_path = tr.to_pathbuf("boot.scr");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_boot_config(&boot_config_file_path, &image_path, true).is_ok()
    );
}
