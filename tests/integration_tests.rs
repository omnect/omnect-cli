mod common;
use common::Testrunner;
use ics_dm_cli::{docker, img_to_bmap_path};
use ics_dm_crypto;
use stdext::function_name;
#[macro_use]
extern crate lazy_static;

#[test]
fn check_set_wifi_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(&config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_wifi_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(&config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_wifi_template_simple() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.simple.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(&config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_wifi_template_simple_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.simple.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(
            &config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_enrollment_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("conf/enrollment_static.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_enrollment_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("conf/enrollment_static.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_enrollment_config(
            &enrollment_config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_gateway_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iotedge-gateway.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("testfiles/full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("testfiles/device-ca.key.pem");

    assert_eq!(
        true,
        docker::set_iotedge_gateway_config(
            &config_file_path,
            &image_path,
            &root_ca_file_path,
            &edge_device_identity_full_chain_file_path,
            &edge_device_identity_key_file_path,
            None
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_gateway_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iotedge-gateway.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("testfiles/full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("testfiles/device-ca.key.pem");

    assert_eq!(
        true,
        docker::set_iotedge_gateway_config(
            &config_file_path,
            &image_path,
            &root_ca_file_path,
            &edge_device_identity_full_chain_file_path,
            &edge_device_identity_key_file_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_leaf_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iot-leaf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");

    assert_eq!(
        true,
        docker::set_iot_leaf_sas_config(&config_file_path, &image_path, &root_ca_file_path, None)
            .is_ok()
    );
}

#[test]
fn check_set_identity_leaf_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iot-leaf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");

    assert_eq!(
        true,
        docker::set_iot_leaf_sas_config(
            &config_file_path,
            &image_path,
            &root_ca_file_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_config_est_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(&config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_identity_config_est_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(
            &config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_config_iot_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iot.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(&config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_identity_config_iot_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iot.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(
            &config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_config_iotedge_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iotedge.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(&config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_identity_config_iotedge_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.ics-iotedge.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(
            &config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_device_cert() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let intermediate_full_chain_crt_path = tr.to_pathbuf("testfiles/test-int-ca_fullchain.pem");
    let intermediate_full_chain_crt_key_path = tr.to_pathbuf("testfiles/test-int-ca.key");

    let intermediate_full_chain_crt = std::fs::read_to_string(&intermediate_full_chain_crt_path)
        .expect("could not read intermediate full-chain-certificate");
    let intermediate_full_chain_crt_key =
        std::fs::read_to_string(&intermediate_full_chain_crt_key_path)
            .expect("could not read intermediate certificate key");

    let crypto = ics_dm_crypto::Crypto::new(
        intermediate_full_chain_crt_key.as_bytes(),
        intermediate_full_chain_crt.as_bytes(),
    )
    .expect("could not create crypto");

    let (device_cert_pem, device_key_pem) = crypto
        .create_cert_and_key("bla", &None, 1)
        .expect("could not create new device certificate");

    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_device_cert(
            &intermediate_full_chain_crt_path,
            &device_cert_pem,
            &device_key_pem,
            &image_path,
            None
        )
        .is_ok()
    );
}

#[test]
fn check_set_iot_hub_device_update_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("conf/du-config.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_iot_hub_device_update_config(&adu_config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_iot_hub_device_update_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("conf/du-config.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_iot_hub_device_update_config(
            &adu_config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}

#[test]
fn check_set_boot_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let boot_config_file_path = tr.to_pathbuf("testfiles/boot.scr");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_boot_config(&boot_config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_boot_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let boot_config_file_path = tr.to_pathbuf("testfiles/boot.scr");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert_eq!(
        true,
        docker::set_boot_config(
            &boot_config_file_path,
            &image_path,
            img_to_bmap_path!(true, &image_path)
        )
        .is_ok()
    );
}
