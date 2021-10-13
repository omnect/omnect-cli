mod common;
use common::Testrunner;
use ics_dm_cli::docker;
use stdext::function_name;

#[test]
fn check_set_wifi_config() {
    let tr = Testrunner::new("check_set_wifi_config");

    let config_file_path = tr.to_pathbuf("wpa_supplicant.conf");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_wifi_config(&config_file_path, &image_path).is_ok()
    );
}

#[test]
fn check_set_enrollment_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path).is_ok()
    );
}

#[test]
fn check_set_enrollment_config_missing_dps() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let enrollment_config_file_path = tr.to_pathbuf("enrollment_static_missing_dps.json");
    let image_path = tr.to_pathbuf("image.wic");

    assert_ne!(
        None,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path)
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
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path)
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

    assert_ne!(
        None,
        docker::set_enrollment_config(&enrollment_config_file_path, &image_path)
            .unwrap_err()
            .to_string()
            .find("unknown field `someKeyWeDontKnow`")
    );
}

#[test]
fn check_set_identity_gateway_config() {
    let tr = Testrunner::new("check_set_identity_gateway_config");

    let config_file_path = tr.to_pathbuf("config.toml");
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
            &edge_device_identity_key_file_path
        )
        .is_ok()
    );
}

#[test]
fn check_set_identity_leaf_config() {
    let tr = Testrunner::new("check_set_identity_leaf_config");

    let config_file_path = tr.to_pathbuf("config.toml");
    let image_path = tr.to_pathbuf("image.wic");
    let root_ca_file_path = tr.to_pathbuf("root.ca.cert.pem");

    assert_eq!(
        true,
        docker::set_iot_leaf_sas_config(&config_file_path, &image_path, &root_ca_file_path).is_ok()
    );
}

#[test]
fn check_set_identity_config() {
    let tr = Testrunner::new("check_set_identity_config");

    let config_file_path = tr.to_pathbuf("config.toml");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_identity_config(&config_file_path, &image_path).is_ok()
    );
}

#[test]
fn check_set_iot_hub_device_update_config() {
    let tr = Testrunner::new(&"check_set_iot_hub_device_update_config");

    let adu_config_file_path = tr.to_pathbuf("adu-conf.txt");
    let image_path = tr.to_pathbuf("image.wic");

    assert_eq!(
        true,
        docker::set_iot_hub_device_update_config(&adu_config_file_path, &image_path).is_ok()
    );
}
