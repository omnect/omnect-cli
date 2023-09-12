mod common;
use common::Testrunner;
use omnect_cli::{cli, docker, img_to_bmap_path, ssh};

use httpmock::prelude::*;

use stdext::function_name;
#[macro_use]
extern crate lazy_static;

#[test]
fn check_set_wifi_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_wifi_config(&config_file_path, &image_path, None).is_ok());
}

#[test]
fn check_set_wifi_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_wifi_config(&config_file_path, &image_path, None).is_ok());
}

#[test]
fn check_set_wifi_template_simple() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.simple.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_wifi_config(&config_file_path, &image_path, None).is_ok());
}

#[test]
fn check_set_wifi_template_simple_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/wpa_supplicant.conf.simple.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_wifi_config(
        &config_file_path,
        &image_path,
        img_to_bmap_path!(true, &image_path)
    )
    .is_ok());
}

#[test]
fn check_set_identity_gateway_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.gateway.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("testfiles/full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("testfiles/device-ca.key.pem");

    assert!(docker::set_iotedge_gateway_config(
        &config_file_path,
        &image_path,
        &root_ca_file_path,
        &edge_device_identity_full_chain_file_path,
        &edge_device_identity_key_file_path,
        None
    )
    .is_ok());
}

#[test]
fn check_set_identity_gateway_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.gateway.tpm.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("testfiles/full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("testfiles/device-ca.key.pem");

    assert!(docker::set_iotedge_gateway_config(
        &config_file_path,
        &image_path,
        &root_ca_file_path,
        &edge_device_identity_full_chain_file_path,
        &edge_device_identity_key_file_path,
        img_to_bmap_path!(true, &image_path)
    )
    .is_ok());
}

#[test]
fn check_set_identity_leaf_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.iot-leaf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");

    assert!(docker::set_iot_leaf_sas_config(
        &config_file_path,
        &image_path,
        &root_ca_file_path,
        None
    )
    .is_ok());
}

#[test]
fn check_set_identity_leaf_config_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.iot-leaf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");

    assert!(docker::set_iot_leaf_sas_config(
        &config_file_path,
        &image_path,
        &root_ca_file_path,
        img_to_bmap_path!(true, &image_path)
    )
    .is_ok());
}

#[test]
fn check_set_identity_config_est_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_identity_config(&config_file_path, &image_path, None, None).is_ok());
}

#[test]
fn check_set_identity_config_payload_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.dsp-payload.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let payload_path = tr.to_pathbuf("testfiles/dps-payload.json");

    assert!(
        docker::set_identity_config(&config_file_path, &image_path, None, Some(payload_path))
            .is_ok()
    );
}

#[test]
fn check_set_identity_config_est_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_identity_config(
        &config_file_path,
        &image_path,
        img_to_bmap_path!(true, &image_path),
        None
    )
    .is_ok());
}

#[test]
fn check_set_identity_config_tpm_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.tpm.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_identity_config(&config_file_path, &image_path, None, None).is_ok());
}

#[test]
fn check_set_identity_config_tpm_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.tpm.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_identity_config(
        &config_file_path,
        &image_path,
        img_to_bmap_path!(true, &image_path),
        None
    )
    .is_ok());
}

#[test]
fn check_set_device_cert() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let intermediate_full_chain_crt_path = tr.to_pathbuf("testfiles/test-int-ca_fullchain.pem");
    let intermediate_full_chain_crt_key_path = tr.to_pathbuf("testfiles/test-int-ca.key");

    let intermediate_full_chain_crt = std::fs::read_to_string(&intermediate_full_chain_crt_path)
        .expect("could not read intermediate full-chain-certificate");
    let intermediate_full_chain_crt_key =
        std::fs::read_to_string(intermediate_full_chain_crt_key_path)
            .expect("could not read intermediate certificate key");

    let crypto = omnect_crypto::Crypto::new(
        intermediate_full_chain_crt_key.as_bytes(),
        intermediate_full_chain_crt.as_bytes(),
    )
    .expect("could not create crypto");

    let (device_cert_pem, device_key_pem) = crypto
        .create_cert_and_key("bla", &None, 1)
        .expect("could not create new device certificate");

    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_device_cert(
        &intermediate_full_chain_crt_path,
        &device_cert_pem,
        &device_key_pem,
        &image_path,
        None
    )
    .is_ok());
}

#[test]
fn check_set_iot_hub_device_update_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("conf/du-config.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(
        docker::set_iot_hub_device_update_config(&adu_config_file_path, &image_path, None).is_ok()
    );
}

#[test]
fn check_set_iot_hub_device_update_template_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("conf/du-config.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::set_iot_hub_device_update_config(
        &adu_config_file_path,
        &image_path,
        img_to_bmap_path!(true, &image_path)
    )
    .is_ok());
}

#[test]
fn check_file_copy() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let boot_config_file_path = tr.to_pathbuf("testfiles/boot.scr");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::file_copy(
        &boot_config_file_path,
        &image_path,
        cli::Partition::boot,
        String::from("/test/test.scr"),
        None
    )
    .is_ok());
}

#[test]
fn check_file_copy_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let boot_config_file_path = tr.to_pathbuf("testfiles/boot.scr");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    assert!(docker::file_copy(
        &boot_config_file_path,
        &image_path,
        cli::Partition::boot,
        String::from("/test/test.scr"),
        img_to_bmap_path!(true, &image_path)
    )
    .is_ok());
}

#[tokio::test]
async fn check_ssh_tunnel_setup() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let mock_access_token = oauth2::AccessToken::new("test_token_mock".to_string());

    let mut config =
        ssh::Config::new("test-backend".to_string(), Some(tr.pathbuf()), None, None).unwrap();

    let server = MockServer::start();

    let request_reply = r#"{
	"clientBastionCert": "-----BEGIN CERTIFICATE-----\nMIIFrjCCA5agAwIBAgIBATANBgkqhkiG...",
	"clientDeviceCert": "-----BEGIN CERTIFICATE-----\nMIIFrjCCA5agAwIBAgIBATANBgkqhkiG...",
	"host": "132.23.0.1",
	"port": 22,
	"bastionUser": "bastion_user"
}
"#;

    let _ = server.mock(|when, then| {
        when.method(POST)
            .path("/api/devices/prepareSSHConnection")
            .header("authorization", "Bearer test_token_mock");
        then.status(200)
            .header("content-type", "application/json")
            .body(request_reply);
    });

    config.set_backend(url::Url::parse(&server.base_url()).unwrap());

    ssh::ssh_create_tunnel("test_device", "test_user", config, mock_access_token)
        .await
        .unwrap();

    assert!(tr.pathbuf().join("ssh_config").exists());
    assert!(tr.pathbuf().join("id_ed25519").exists());
    assert!(tr.pathbuf().join("id_ed25519.pub").exists());
    assert!(tr.pathbuf().join("bastion-cert.pub").exists());
    assert!(tr.pathbuf().join("device-cert.pub").exists());

    let ssh_config = std::fs::read_to_string(tr.pathbuf().join("ssh_config")).unwrap();
    let expected_config = format!(
        r#"Host bastion
	User bastion_user
	Hostname 132.23.0.1
	Port 22
	IdentityFile {}/id_ed25519
	CertificateFile {}/bastion-cert.pub
	ProxyCommand none

Host test_device
	User test_user
	IdentityFile {}/id_ed25519
	CertificateFile {}/device-cert.pub
	ProxyCommand ssh -F {}/ssh_config bastion
"#,
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy()
    );

    assert_eq!(ssh_config, expected_config);
}
