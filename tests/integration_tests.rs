mod common;
use assert_cmd::Command;
use assert_json_diff::assert_json_eq;
use common::Testrunner;
use httpmock::prelude::*;
use omnect_cli::ssh;
use std::{fs::create_dir_all, path::PathBuf};
use stdext::function_name;

#[macro_use]
extern crate lazy_static;

#[test]
fn check_set_identity_gateway_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let config_file_path = tr.to_pathbuf("conf/config.toml.gateway.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");
    let edge_device_identity_full_chain_file_path = tr.to_pathbuf("testfiles/full-chain.cert.pem");
    let edge_device_identity_key_file_path = tr.to_pathbuf("testfiles/device-ca.key.pem");

    let mut set_iotedge_gateway_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_iotedge_gateway_config
        .arg("identity")
        .arg("set-iotedge-gateway-config")
        .arg("-c")
        .arg(&config_file_path)
        .arg("-i")
        .arg(&image_path)
        .arg("-r")
        .arg(&root_ca_file_path)
        .arg("-d")
        .arg(&edge_device_identity_full_chain_file_path)
        .arg("-k")
        .arg(&edge_device_identity_key_file_path)
        .assert();
    assert.success();

    let mut config_file_out_path = tr.pathbuf();
    config_file_out_path.push("dir1");
    create_dir_all(config_file_out_path.clone()).unwrap();
    let mut root_ca_file_out_path = config_file_out_path.clone();
    let mut edge_device_identity_full_chain_file_out_path = config_file_out_path.clone();
    let mut edge_device_identity_key_file_out_path = config_file_out_path.clone();
    let mut hosts_file_out_path = config_file_out_path.clone();
    let mut hostname_file_out_path = config_file_out_path.clone();

    config_file_out_path.push("config_file_out_path");
    let config_file_out_path = config_file_out_path.to_str().unwrap();

    root_ca_file_out_path.push("root_ca_file_out_path");
    let root_ca_file_out_path = root_ca_file_out_path.to_str().unwrap();

    edge_device_identity_full_chain_file_out_path
        .push("edge_device_identity_full_chain_file_out_path");
    let edge_device_identity_full_chain_file_out_path =
        edge_device_identity_full_chain_file_out_path
            .to_str()
            .unwrap();

    edge_device_identity_key_file_out_path.push("edge_device_identity_key_file_out_path");
    let edge_device_identity_key_file_out_path =
        edge_device_identity_key_file_out_path.to_str().unwrap();
    hosts_file_out_path.push("hosts_file_out_path");
    let hosts_file_out_path = hosts_file_out_path.to_str().unwrap();
    hostname_file_out_path.push("hostname_file_out_path");
    let hostname_file_out_path = hostname_file_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/etc/aziot/config.toml,{config_file_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "cert:/ca/trust-bundle.pem.crt,{root_ca_file_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "cert:/priv/edge-ca.pem,{edge_device_identity_full_chain_file_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "cert:/priv/edge-ca.key.pem,{edge_device_identity_key_file_out_path}"
        ))
        .arg("-f")
        .arg(format!("factory:/etc/hosts,{hosts_file_out_path}"))
        .arg("-f")
        .arg(format!("factory:/etc/hostname,{hostname_file_out_path}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        config_file_path.to_str().unwrap(),
        config_file_out_path
    ));
    assert!(file_diff::diff(
        root_ca_file_path.to_str().unwrap(),
        root_ca_file_out_path
    ));
    assert!(file_diff::diff(
        edge_device_identity_full_chain_file_path.to_str().unwrap(),
        edge_device_identity_full_chain_file_out_path
    ));
    assert!(file_diff::diff(
        edge_device_identity_key_file_path.to_str().unwrap(),
        edge_device_identity_key_file_out_path
    ));
    assert!(std::path::Path::new(hosts_file_out_path)
        .try_exists()
        .is_ok_and(|exists| exists));

    assert!(std::fs::read_to_string(hosts_file_out_path)
        .unwrap()
        .contains("127.0.1.1 my-omnect-iotedge-gateway-device"));

    assert!(std::fs::read_to_string(hostname_file_out_path)
        .unwrap()
        .contains("my-omnect-iotedge-gateway-device"));
}

#[test]
fn check_set_identity_leaf_config() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let config_file_path = tr.to_pathbuf("conf/config.toml.iot-leaf.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let root_ca_file_path = tr.to_pathbuf("testfiles/root.ca.cert.pem");

    let mut set_iot_leaf_sas_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_iot_leaf_sas_config
        .arg("identity")
        .arg("set-iot-leaf-sas-config")
        .arg("-c")
        .arg(&config_file_path)
        .arg("-i")
        .arg(&image_path)
        .arg("-r")
        .arg(&root_ca_file_path)
        .assert();
    assert.success();

    let mut config_file_out_path = tr.pathbuf();
    config_file_out_path.push("dir1");
    create_dir_all(config_file_out_path.clone()).unwrap();
    let mut root_ca_file_out_path = config_file_out_path.clone();
    let mut hosts_file_out_path = config_file_out_path.clone();
    let mut hostname_file_out_path = config_file_out_path.clone();

    config_file_out_path.push("config_file_out_path");
    let config_file_out_path = config_file_out_path.to_str().unwrap();
    root_ca_file_out_path.push("root_ca_file_out_path");
    let root_ca_file_out_path = root_ca_file_out_path.to_str().unwrap();
    hosts_file_out_path.push("hosts_file_out_path");
    let hosts_file_out_path = hosts_file_out_path.to_str().unwrap();
    hostname_file_out_path.push("hostname_file_out_path");
    let hostname_file_out_path = hostname_file_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/etc/aziot/config.toml,{config_file_out_path}"
        ))
        .arg("-f")
        .arg(format!("cert:/ca/root.ca.cert.crt,{root_ca_file_out_path}"))
        .arg("-f")
        .arg(format!("factory:/etc/hosts,{hosts_file_out_path}"))
        .arg("-f")
        .arg(format!("factory:/etc/hostname,{hostname_file_out_path}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        config_file_path.to_str().unwrap(),
        config_file_out_path
    ));
    assert!(file_diff::diff(
        root_ca_file_path.to_str().unwrap(),
        root_ca_file_out_path
    ));

    assert!(std::fs::read_to_string(hosts_file_out_path)
        .unwrap()
        .contains("127.0.1.1 my-omnect-iot-leaf-device"));

    assert!(std::fs::read_to_string(hostname_file_out_path)
        .unwrap()
        .contains("my-omnect-iot-leaf-device"));
}

#[test]
fn check_set_identity_config_est_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    let mut set_identity_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_identity_config
        .arg("identity")
        .arg("set-config")
        .arg("-c")
        .arg(&config_file_path)
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let mut config_file_out_path = tr.pathbuf();
    config_file_out_path.push("dir1");
    create_dir_all(config_file_out_path.clone()).unwrap();
    let mut hosts_file_out_path = config_file_out_path.clone();
    let mut hostname_file_out_path = config_file_out_path.clone();

    config_file_out_path.push("config_file_out_path");
    let config_file_out_path = config_file_out_path.to_str().unwrap();
    hosts_file_out_path.push("hosts_file_out_path");
    let hosts_file_out_path = hosts_file_out_path.to_str().unwrap();
    hostname_file_out_path.push("hostname_file_out_path");
    let hostname_file_out_path = hostname_file_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/etc/aziot/config.toml,{config_file_out_path}"
        ))
        .arg("-f")
        .arg(format!("factory:/etc/hosts,{hosts_file_out_path}"))
        .arg("-f")
        .arg(format!("factory:/etc/hostname,{hostname_file_out_path}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        config_file_path.to_str().unwrap(),
        config_file_out_path
    ));

    assert!(std::fs::read_to_string(hosts_file_out_path)
        .unwrap()
        .contains("127.0.1.1 test-omnect-est"));

    assert!(std::fs::read_to_string(hostname_file_out_path)
        .unwrap()
        .contains("test-omnect-est"));
}

#[test]
fn check_set_identity_config_payload_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.est.dps-payload.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let payload_path = tr.to_pathbuf("testfiles/dps-payload.json");

    let mut set_iot_leaf_sas_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_iot_leaf_sas_config
        .arg("identity")
        .arg("set-config")
        .arg("-c")
        .arg(&config_file_path)
        .arg("-i")
        .arg(&image_path)
        .arg("-e")
        .arg(&payload_path)
        .assert();
    assert.success();

    let mut config_file_out_path = tr.pathbuf();
    config_file_out_path.push("dir1");
    create_dir_all(config_file_out_path.clone()).unwrap();
    let mut payload_out_path = config_file_out_path.clone();
    let mut hosts_file_out_path = config_file_out_path.clone();
    let mut hostname_file_out_path = config_file_out_path.clone();

    config_file_out_path.push("config_file_out_path");
    let config_file_out_path = config_file_out_path.to_str().unwrap();
    payload_out_path.push("root_ca_file_out_path");
    let payload_out_path = payload_out_path.to_str().unwrap();
    hosts_file_out_path.push("hosts_file_out_path");
    let hosts_file_out_path = hosts_file_out_path.to_str().unwrap();
    hostname_file_out_path.push("hostname_file_out_path");
    let hostname_file_out_path = hostname_file_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/etc/aziot/config.toml,{config_file_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "factory:/etc/omnect/dps-payload.json,{payload_out_path}"
        ))
        .arg("-f")
        .arg(format!("factory:/etc/hosts,{hosts_file_out_path}"))
        .arg("-f")
        .arg(format!("factory:/etc/hostname,{hostname_file_out_path}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        config_file_path.to_str().unwrap(),
        config_file_out_path
    ));
    assert!(file_diff::diff(
        payload_path.to_str().unwrap(),
        payload_out_path
    ));

    assert!(std::fs::read_to_string(hosts_file_out_path)
        .unwrap()
        .contains("127.0.1.1 test-omnect-est-with-payload"));

    assert!(std::fs::read_to_string(hostname_file_out_path)
        .unwrap()
        .contains("test-omnect-est-with-payload"));
}

#[test]
fn check_set_identity_config_tpm_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let config_file_path = tr.to_pathbuf("conf/config.toml.tpm.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    let mut set_identity_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_identity_config
        .arg("identity")
        .arg("set-config")
        .arg("-c")
        .arg(&config_file_path)
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let mut config_file_out_path = tr.pathbuf();
    config_file_out_path.push("dir1");
    create_dir_all(config_file_out_path.clone()).unwrap();
    let mut hosts_file_out_path = config_file_out_path.clone();
    let mut hostname_file_out_path = config_file_out_path.clone();

    config_file_out_path.push("config_file_out_path");
    let config_file_out_path = config_file_out_path.to_str().unwrap();
    hosts_file_out_path.push("hosts_file_out_path");
    let hosts_file_out_path = hosts_file_out_path.to_str().unwrap();
    hostname_file_out_path.push("hostname_file_out_path");
    let hostname_file_out_path = hostname_file_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/etc/aziot/config.toml,{config_file_out_path}"
        ))
        .arg("-f")
        .arg(format!("factory:/etc/hosts,{hosts_file_out_path}"))
        .arg("-f")
        .arg(format!("factory:/etc/hostname,{hostname_file_out_path}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        config_file_path.to_str().unwrap(),
        config_file_out_path
    ));

    assert!(std::fs::read_to_string(hosts_file_out_path)
        .unwrap()
        .contains("127.0.1.1 my-omnect-iot-tpm-device"));

    assert!(std::fs::read_to_string(hostname_file_out_path)
        .unwrap()
        .contains("my-omnect-iot-tpm-device"));
}

#[test]
fn check_set_device_cert() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let intermediate_full_chain_crt_path = tr.to_pathbuf("testfiles/test-int-ca_fullchain.pem");
    let intermediate_full_chain_crt_key_path = tr.to_pathbuf("testfiles/test-int-ca.key");

    let mut set_device_certificate = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_device_certificate
        .arg("identity")
        .arg("set-device-certificate")
        .arg("-c")
        .arg(&intermediate_full_chain_crt_path)
        .arg("-k")
        .arg(&intermediate_full_chain_crt_key_path)
        .arg("-i")
        .arg(&image_path)
        .arg("-d")
        .arg("my-device-id")
        .arg("-D")
        .arg("1")
        .assert();
    assert.success();

    let mut device_id_cert_out_path = tr.pathbuf();
    device_id_cert_out_path.push("dir1");
    create_dir_all(device_id_cert_out_path.clone()).unwrap();

    let mut device_id_cert_key_out_path = device_id_cert_out_path.clone();
    let mut ca_crt_pem_out_path = device_id_cert_out_path.clone();
    let mut ca_pem_out_path = device_id_cert_out_path.clone();

    device_id_cert_out_path.push("device_id_cert_out_path");
    let device_id_cert_out_path = device_id_cert_out_path.to_str().unwrap();

    device_id_cert_key_out_path.push("device_id_cert_key_out_path");
    let device_id_cert_key_out_path = device_id_cert_key_out_path.to_str().unwrap();

    ca_crt_pem_out_path.push("ca_crt_pem_out_path");
    let ca_crt_pem_out_path = ca_crt_pem_out_path.to_str().unwrap();

    ca_pem_out_path.push("ca_pem_out_path");
    let ca_pem_out_path = ca_pem_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "cert:/priv/device_id_cert.pem,{device_id_cert_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "cert:/priv/device_id_cert_key.pem,{device_id_cert_key_out_path}"
        ))
        .arg("-f")
        .arg(format!("cert:/priv/ca.crt.pem,{ca_crt_pem_out_path}"))
        .arg("-f")
        .arg(format!("cert:/ca/ca.crt,{ca_pem_out_path}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        intermediate_full_chain_crt_path.to_str().unwrap(),
        ca_crt_pem_out_path
    ));
}

#[test]
fn check_set_iot_hub_device_update_template() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let adu_config_file_path = tr.to_pathbuf("conf/du-config.json.template");
    let image_path = tr.to_pathbuf("testfiles/image.wic");

    let mut set_iot_hub_device_update_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_iot_hub_device_update_config
        .arg("iot-hub-device-update")
        .arg("set-device-config")
        .arg("-c")
        .arg(&adu_config_file_path)
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let mut adu_config_file_out_path = tr.pathbuf();
    adu_config_file_out_path.push("dir1");
    create_dir_all(adu_config_file_out_path.clone()).unwrap();
    adu_config_file_out_path.push("adu_config_file_out_path");
    let adu_config_file_out_path = adu_config_file_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/etc/adu/du-config.json,{adu_config_file_out_path}"
        ))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        adu_config_file_path.to_str().unwrap(),
        adu_config_file_out_path
    ));
}

#[test]
fn check_set_iot_hub_device_update_create_import_manifest() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let image_path = tr.to_pathbuf("testfiles/image.swu");
    let script_path = tr.to_pathbuf("testfiles/image.swu.sh");
    let manifest_created = script_path.with_extension("importManifest.json");
    let manifest_original = tr.to_pathbuf("testfiles/image.swu.importManifest.json.orig");

    let mut set_iot_hub_device_update_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_iot_hub_device_update_config
        .current_dir(tr.pathbuf())
        .arg("iot-hub-device-update")
        .arg("create-import-manifest")
        .arg("-d")
        .arg("OMNECT-gateway-devel")
        .arg("-v")
        .arg("4.0.15.0")
        .arg("-i")
        .arg(&image_path)
        .arg("-s")
        .arg(&script_path)
        .arg("-n")
        .arg("omnect-raspberrypi4-64-gateway-devel")
        .arg("-c")
        .arg("2")
        .assert();
    assert.success();

    let mut manifest_created: serde_json::Value = serde_json::from_reader(
        std::fs::OpenOptions::new()
            .read(true)
            .create(false)
            .open(manifest_created)
            .unwrap(),
    )
    .unwrap();

    manifest_created["createdDateTime"] = serde_json::json!("removed");

    let manifest_original: serde_json::Value = serde_json::from_reader(
        std::fs::OpenOptions::new()
            .read(true)
            .create(false)
            .open(manifest_original)
            .unwrap(),
    )
    .unwrap();

    assert_json_eq!(manifest_created, manifest_original);
}

#[test]
fn check_file_copy_dos_partition() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    check_file_copy(tr, "boot");
}

#[test]
fn check_file_copy_ext4() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    check_file_copy(tr, "factory");
}

fn check_file_copy(tr: Testrunner, partition: &str) {
    let in_file1 = tr.to_pathbuf("testfiles/boot.scr");
    let in_file1 = in_file1.to_str().unwrap();
    let in_file2 = tr.to_pathbuf("testfiles/dps-payload.json");
    let in_file2 = in_file2.to_str().unwrap();
    let mut out_file1 = tr.pathbuf();
    out_file1.push("test/test1.scr");
    create_dir_all(out_file1.parent().unwrap()).unwrap();
    let out_file1 = out_file1.to_str().unwrap();
    let mut out_file2 = tr.pathbuf();
    out_file2.push("test2.json");
    let out_file2 = out_file2.to_str().unwrap();
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let mut out_file3 = tr.pathbuf();
    out_file3.push("dir1");
    out_file3.push("outfile3.scr");
    create_dir_all(out_file3.parent().unwrap()).unwrap();
    let out_file3 = out_file3.to_str().unwrap();
    let mut out_file4 = tr.pathbuf();
    out_file4.push("outfile4.json");
    let out_file4 = out_file4.to_str().unwrap();

    let image_path_hash1 = Testrunner::file_hash(&image_path);

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file1},{partition}:{out_file1}"))
        .arg("-f")
        .arg(format!("{in_file2},{partition}:{out_file2}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let image_path_hash2 = Testrunner::file_hash(&image_path);

    assert_ne!(image_path_hash1, image_path_hash2);

    let image_path_hash1 = Testrunner::file_hash(&image_path);

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!("{partition}:{out_file1},{out_file3}"))
        .arg("-f")
        .arg(format!("{partition}:{out_file2},{out_file4}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let image_path_hash2 = Testrunner::file_hash(&image_path);

    assert_eq!(image_path_hash1, image_path_hash2);

    assert!(file_diff::diff(in_file1, out_file3));
    assert!(file_diff::diff(in_file2, out_file4));

    // test if we can overwrite files
    let in_file3 = tr.to_pathbuf("testfiles/identity_config_dps_payload.toml");
    let in_file3 = in_file3.to_str().unwrap();
    let in_file4 = tr.to_pathbuf("testfiles/identity_config_hostname_valid.toml");
    let in_file4 = in_file4.to_str().unwrap();

    let image_path_hash1 = Testrunner::file_hash(&image_path);

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file3},{partition}:{out_file1}"))
        .arg("-f")
        .arg(format!("{in_file4},{partition}:{out_file2}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let image_path_hash2 = Testrunner::file_hash(&image_path);

    assert_ne!(image_path_hash1, image_path_hash2);

    let image_path_hash1 = Testrunner::file_hash(&image_path);

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!("{partition}:{out_file1},{out_file3}"))
        .arg("-f")
        .arg(format!("{partition}:{out_file2},{out_file4}"))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let image_path_hash2 = Testrunner::file_hash(&image_path);

    assert_eq!(image_path_hash1, image_path_hash2);
    assert!(file_diff::diff(in_file3, out_file3));
    assert!(file_diff::diff(in_file4, out_file4));
}

#[test]
fn check_bmap_generation_wic() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let image_path_copy = PathBuf::from(format!("{}.copy", image_path.to_str().unwrap()));
    let bmap_path = PathBuf::from(format!("{}.bmap", image_path.to_str().unwrap()));
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let in_file = in_file.to_str().unwrap();

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},boot:/my-file"))
        .arg("-i")
        .arg(&image_path)
        .assert();

    assert.success();

    assert!(!bmap_path.try_exists().is_ok_and(|exists| exists));

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},boot:/my-file"))
        .arg("-i")
        .arg(&image_path)
        .arg("-b")
        .assert();
    assert.success();

    assert!(bmap_path.try_exists().is_ok_and(|exists| exists));

    // use bmaptool to verify that the checksum of the bmap file and the image
    // still match after the copy operations
    let assert = Command::new("bmaptool")
        .arg("copy")
        .args(["--bmap", &bmap_path.to_string_lossy()])
        .arg(&image_path)
        .arg(&image_path_copy)
        .assert();
    assert.success();
}

#[test]
fn check_bmap_generation_wic_xz() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path_wic_xz = tr.to_pathbuf("testfiles/image.wic.xz");
    let image_path_wic = image_path_wic_xz.with_extension("");
    let image_path_wic_copy = image_path_wic_xz.with_extension("copy");
    let image_path_bmap = image_path_wic_xz.with_extension("bmap");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let in_file = in_file.to_str().unwrap();

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},boot:/my-file"))
        .arg("-i")
        .arg(&image_path_wic_xz)
        .arg("-b")
        .assert();
    assert.success();

    // use bmaptool to verify that the checksum of the bmap file and the image
    // still match after the copy operations
    let assert = Command::new("bmaptool")
        .arg("copy")
        .args(["--bmap", &image_path_bmap.to_string_lossy()])
        .arg(&image_path_wic)
        .arg(&image_path_wic_copy)
        .assert();
    assert.success();
}

#[test]
fn check_image_compression() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path_wic = tr.to_pathbuf("testfiles/image.wic");
    let image_path_wic_xz = PathBuf::from(format!("{}.xz", image_path_wic.to_str().unwrap()));
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let in_file = in_file.to_str().unwrap();

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},boot:/my-file"))
        .arg("-i")
        .arg(&image_path_wic)
        .assert();
    assert.success();

    assert!(!image_path_wic_xz.try_exists().is_ok_and(|exists| exists));

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},boot:/my-file"))
        .arg("-i")
        .arg(&image_path_wic)
        .arg("-p")
        .arg("xz")
        .assert();
    assert.success();

    let image_path_wic_xz_hash1 = Testrunner::file_hash(&image_path_wic_xz);

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},factory:/my-file"))
        .arg("-i")
        .arg(&image_path_wic_xz)
        .arg("-p")
        .arg("xz")
        .assert();
    assert.success();

    let image_path_wic_xz_hash2 = Testrunner::file_hash(&image_path_wic_xz);

    assert_ne!(image_path_wic_xz_hash1, image_path_wic_xz_hash2);
}

#[test]
fn check_image_decompression() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path_wic_xz = tr.to_pathbuf("testfiles/image.wic.xz");
    let image_path_wic = image_path_wic_xz.with_extension("");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let in_file = in_file.to_str().unwrap();
    let image_path_wic_xz_hash1 = Testrunner::file_hash(&image_path_wic_xz);

    let mut copy_to_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_to_img
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{in_file},boot:/my-file"))
        .arg("-i")
        .arg(&image_path_wic_xz)
        .assert();
    assert.success();

    assert!(image_path_wic.try_exists().is_ok_and(|exists| exists));

    let image_path_wic_xz_hash2 = Testrunner::file_hash(&image_path_wic_xz);

    assert_eq!(image_path_wic_xz_hash1, image_path_wic_xz_hash2);
}

#[tokio::test]
async fn check_ssh_tunnel_setup() {
    let tr = Testrunner::new("check_ssh_tunnel_setup");

    let mock_access_token = oauth2::AccessToken::new("test_token_mock".to_string());

    let mut config = ssh::Config::new("test-backend", Some(tr.pathbuf()), None, None).unwrap();

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

    assert!(tr
        .pathbuf()
        .join("config")
        .try_exists()
        .is_ok_and(|exists| exists));
    assert!(tr
        .pathbuf()
        .join("id_ed25519")
        .try_exists()
        .is_ok_and(|exists| exists));
    assert!(tr
        .pathbuf()
        .join("id_ed25519.pub")
        .try_exists()
        .is_ok_and(|exists| exists));
    assert!(tr
        .pathbuf()
        .join("bastion-cert.pub")
        .try_exists()
        .is_ok_and(|exists| exists));
    assert!(tr
        .pathbuf()
        .join("device-cert.pub")
        .try_exists()
        .is_ok_and(|exists| exists));

    let ssh_config = std::fs::read_to_string(tr.pathbuf().join("config")).unwrap();
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
	ProxyCommand ssh -F {}/config bastion
"#,
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy(),
        tr.pathbuf().to_string_lossy()
    );

    assert_eq!(ssh_config, expected_config);
}

#[tokio::test]
async fn check_existing_ssh_config_not_overwritten() {
    let tr = Testrunner::new("check_existing_ssh_config_not_overwritten");

    let mock_access_token = oauth2::AccessToken::new("test_token_mock".to_string());

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

    let mut config_path = tr.pathbuf();
    config_path.push("config");

    let config_content_before = "some_test_data";
    std::fs::write(&config_path, config_content_before).unwrap();

    let mut config = ssh::Config::new(
        "test-backend",
        Some(tr.pathbuf()),
        None,
        Some(config_path.clone()),
    )
    .unwrap();

    config.set_backend(url::Url::parse(&server.base_url()).unwrap());

    let result =
        ssh::ssh_create_tunnel("test_device", "test_user", config, mock_access_token).await;

    assert!(matches!(result, Result::Err(_)));

    assert_eq!(
        result.err().unwrap().to_string(),
        r#"ssh config file already exists and would be overwritten.
Please remove config file first."#
    );

    let config_content_after = std::fs::read_to_string(&config_path).unwrap();

    assert_eq!(config_content_before, &config_content_after);
}

#[test]
fn check_docker_inject_image_success() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let image_path = tr.to_pathbuf("testfiles/image.wic");

    let mut docker_inject_image_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = docker_inject_image_config
        .arg("docker")
        .arg("inject")
        .args(["--docker-image", "some-image"])
        .args(["--image", &image_path.to_string_lossy()])
        .args(["--partition", "factory"])
        .args(["--dest", "/some/test/dir"])
        .assert();
    let result_output = String::from_utf8(assert.get_output().stdout.to_vec()).unwrap();
    let result_output = result_output.trim();
    assert.success();

    const EXPECTED_OUTPUT: &str = "Stored some-image to factory:/some/test/dir/some-image.tar.gz";

    assert_eq!(EXPECTED_OUTPUT, result_output);

    let mut docker_image_out_path = tr.pathbuf();
    docker_image_out_path.push("docker");
    create_dir_all(docker_image_out_path.clone()).unwrap();
    docker_image_out_path.push("some-image.tar.gz");

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "factory:/some/test/dir/some-image.tar.gz,{}",
            docker_image_out_path.to_string_lossy(),
        ))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    const EXPECTED_CONTENT: &str = "some test data";

    let result_content = std::fs::read_to_string(docker_image_out_path).unwrap();

    assert_eq!(EXPECTED_CONTENT, result_content);
}
