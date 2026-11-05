mod common;
use assert_cmd::Command;
use assert_json_diff::assert_json_eq;
use common::Testrunner;
use httpmock::prelude::*;
use omnect_cli::{
    file::bmap::{Bmap, bmap_path},
    ssh,
};
use predicates::prelude::*;
use std::{
    ffi::OsStr,
    fs::{self, create_dir_all},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use stdext::function_name;

#[test]
fn check_help_root_level() {
    Command::cargo_bin("omnect-cli")
        .expect("binary exists")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("docker"))
        .stdout(predicate::str::contains("identity"))
        .stdout(predicate::str::contains("ssh"));
}

#[test]
fn check_help_subcommand_group() {
    Command::cargo_bin("omnect-cli")
        .expect("binary exists")
        .arg("identity")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("set-config"))
        .stdout(predicate::str::contains("set-device-certificate"));
}

#[test]
fn check_help_leaf_command() {
    Command::cargo_bin("omnect-cli")
        .expect("binary exists")
        .arg("identity")
        .arg("set-config")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--image"));
}

#[test]
fn check_help_short_flag() {
    Command::cargo_bin("omnect-cli")
        .expect("binary exists")
        .arg("identity")
        .arg("set-config")
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"));
}

#[test]
fn check_version_flag() {
    Command::cargo_bin("omnect-cli")
        .expect("binary exists")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("omnect-cli"));
}

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
    assert!(
        std::path::Path::new(hosts_file_out_path)
            .try_exists()
            .is_ok_and(|exists| exists)
    );

    assert!(
        std::fs::read_to_string(hosts_file_out_path)
            .unwrap()
            .contains("127.0.1.1 my-omnect-iotedge-gateway-device")
    );

    assert!(
        std::fs::read_to_string(hostname_file_out_path)
            .unwrap()
            .contains("my-omnect-iotedge-gateway-device")
    );
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

    assert!(
        std::fs::read_to_string(hosts_file_out_path)
            .unwrap()
            .contains("127.0.1.1 my-omnect-iot-leaf-device")
    );

    assert!(
        std::fs::read_to_string(hostname_file_out_path)
            .unwrap()
            .contains("my-omnect-iot-leaf-device")
    );
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

    assert!(
        std::fs::read_to_string(hosts_file_out_path)
            .unwrap()
            .contains("127.0.1.1 test-omnect-est")
    );

    assert!(
        std::fs::read_to_string(hostname_file_out_path)
            .unwrap()
            .contains("test-omnect-est")
    );
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

    assert!(
        std::fs::read_to_string(hosts_file_out_path)
            .unwrap()
            .contains("127.0.1.1 test-omnect-est-with-payload")
    );

    assert!(
        std::fs::read_to_string(hostname_file_out_path)
            .unwrap()
            .contains("test-omnect-est-with-payload")
    );
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

    assert!(
        std::fs::read_to_string(hosts_file_out_path)
            .unwrap()
            .contains("127.0.1.1 test-omnect-tpm")
    );

    assert!(
        std::fs::read_to_string(hostname_file_out_path)
            .unwrap()
            .contains("test-omnect-tpm")
    );
}

/// Fails if a file with one of `names` is in the directory of `image`.
fn assert_no_files_next_to(image: &Path, names: &[&str]) {
    for name in names {
        let path = image.with_file_name(name);
        assert!(!path.exists(), "{} was left behind", path.display());
    }
}

#[test]
fn check_set_device_cert_est() {
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
    assert_no_files_next_to(
        &image_path,
        &["device_cert_path.pem", "device_key_path.key.pem"],
    );

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
fn check_set_device_cert_no_est() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let device_crt_path = tr.to_pathbuf("testfiles/test-int-ca_fullchain.pem");
    let device_crt_key_path = tr.to_pathbuf("testfiles/test-int-ca.key");

    let mut set_device_certificate_no_est = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_device_certificate_no_est
        .arg("identity")
        .arg("set-device-certificate-no-est")
        .arg("-c")
        .arg(&device_crt_path)
        .arg("-k")
        .arg(&device_crt_key_path)
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    let mut device_cert_out_path = tr.pathbuf();
    device_cert_out_path.push("dir1");
    create_dir_all(device_cert_out_path.clone()).unwrap();

    let mut device_cert_key_out_path = device_cert_out_path.clone();

    device_cert_out_path.push("device_id_cert_out_path");
    let device_cert_out_path = device_cert_out_path.to_str().unwrap();

    device_cert_key_out_path.push("device_id_cert_key_out_path");
    let device_cert_key_out_path = device_cert_key_out_path.to_str().unwrap();

    let mut copy_from_img = Command::cargo_bin("omnect-cli").unwrap();
    let assert = copy_from_img
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!(
            "cert:/priv/device_id_cert.pem,{device_cert_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "cert:/priv/device_id_cert_key.pem,{device_cert_key_out_path}"
        ))
        .arg("-i")
        .arg(&image_path)
        .assert();
    assert.success();

    assert!(file_diff::diff(
        device_crt_path.to_str().unwrap(),
        device_cert_out_path
    ));

    assert!(file_diff::diff(
        device_crt_key_path.to_str().unwrap(),
        device_cert_key_out_path
    ));
}

#[test]
fn check_set_edge_ca_cert() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let intermediate_full_chain_crt_path = tr.to_pathbuf("testfiles/test-int-ca_fullchain.pem");
    let intermediate_full_chain_crt_key_path = tr.to_pathbuf("testfiles/test-int-ca.key");

    let mut set_device_certificate = Command::cargo_bin("omnect-cli").unwrap();
    let assert = set_device_certificate
        .arg("identity")
        .arg("set-edge-ca-certificate")
        .arg("-c")
        .arg(&intermediate_full_chain_crt_path)
        .arg("-k")
        .arg(&intermediate_full_chain_crt_key_path)
        .arg("-i")
        .arg(&image_path)
        .arg("-d")
        .arg("edge-ca")
        .arg("-D")
        .arg("1")
        .assert();
    assert.success();
    assert_no_files_next_to(
        &image_path,
        &["edge_ca_cert_path.pem", "edge_ca_key_path.key.pem"],
    );

    let mut edge_ca_cert_out_path = tr.pathbuf();
    edge_ca_cert_out_path.push("dir1");
    create_dir_all(edge_ca_cert_out_path.clone()).unwrap();

    let mut edge_ca_cert_key_out_path = edge_ca_cert_out_path.clone();
    let mut ca_crt_pem_out_path = edge_ca_cert_out_path.clone();
    let mut ca_pem_out_path = edge_ca_cert_out_path.clone();

    edge_ca_cert_out_path.push("edge_ca_cert_out_path");
    let edge_ca_cert_out_path = edge_ca_cert_out_path.to_str().unwrap();

    edge_ca_cert_key_out_path.push("edge_ca_cert_key_out_path");
    let edge_ca_cert_key_out_path = edge_ca_cert_key_out_path.to_str().unwrap();

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
            "cert:/priv/edge_ca_cert.pem,{edge_ca_cert_out_path}"
        ))
        .arg("-f")
        .arg(format!(
            "cert:/priv/edge_ca_cert_key.pem,{edge_ca_cert_key_out_path}"
        ))
        .arg("-f")
        .arg(format!("cert:/priv/edge_ca.crt.pem,{ca_crt_pem_out_path}"))
        .arg("-f")
        .arg(format!("cert:/ca/edge_ca.crt,{ca_pem_out_path}"))
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

// unit of `st_blocks`
const STAT_BLOCK_SIZE: u64 = 512;
// content an earlier installation left on the device
const OLD_CONTENT_BYTE: u8 = 0xa5;
// a file with a zero range that spans whole image blocks
const ZERO_BLOCKS_FILE_EDGE: usize = 4096;
const ZERO_BLOCKS_FILE_ZEROS: usize = 64 * 1024;
// a valid bmap of a one-block image, so only the image size check rejects it
const OTHER_IMAGE_BMAP: &str = "<bmap version=\"2.0\"><ImageSize>4096</ImageSize>\
    <BlockSize>4096</BlockSize><ChecksumType>sha256</ChecksumType><BlockMap>\
    <Range chksum=\"ad7facb2586fc6e966c004d7d1d16b024f5805ff7cb47c7a85dabd8b48892ca7\">0</Range>\
    </BlockMap></bmap>";
const OTHER_IMAGE_SIZE_ERROR: &str = "bmap is for an image of 4096 bytes";

fn read_bmap(path: &Path) -> Bmap {
    Bmap::read(path).expect("read bmap")
}

fn mapped_blocks(bmap: &Bmap) -> u64 {
    bmap.ranges.iter().map(|r| r.end() - r.start() + 1).sum()
}

fn maps_whole_image(bmap: &Bmap) -> bool {
    mapped_blocks(bmap) == bmap.image_size.div_ceil(bmap.block_size)
}

fn maps_all_ranges_of(bmap: &Bmap, input: &Bmap) -> bool {
    input
        .ranges
        .iter()
        .flat_map(|r| r.clone())
        .all(|b| bmap.ranges.iter().any(|r| r.contains(&b)))
}

/// Writes the blocks `bmap` maps onto a device that still holds old content, as a bmap flash
/// does.
fn bmap_flash(image: &[u8], bmap: &Bmap) -> Vec<u8> {
    let mut device = vec![OLD_CONTENT_BYTE; image.len()];
    for range in &bmap.ranges {
        let start = usize::try_from(range.start() * bmap.block_size).expect("start");
        let end = usize::try_from((range.end() + 1) * bmap.block_size).expect("end");
        let end = end.min(image.len());
        device[start..end].copy_from_slice(&image[start..end]);
    }
    device
}

fn sparse_copy(from: &Path, to: &Path) {
    let status = std::process::Command::new("cp")
        .arg("--sparse=always")
        .arg(from)
        .arg(to)
        .status()
        .expect("run cp");
    assert!(status.success(), "sparse copy of the test image failed");
}

#[test]
fn check_file_copy_without_input_bmap_maps_whole_image() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let bmap_path = bmap_path(&image_path);
    let in_file = tr.to_pathbuf("testfiles/boot.scr");

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},factory:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .arg("-b")
        .assert()
        .success();

    assert!(maps_whole_image(&read_bmap(&bmap_path)));
}

#[test]
fn check_file_copy_keeps_holes() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let input_bmap_path = tr.to_pathbuf("testfiles/image.wic.bmap");
    let sparse_path = image_path.with_file_name("sparse.wic");
    let bmap_path = bmap_path(&sparse_path);
    let in_file = tr.to_pathbuf("testfiles/boot.scr");

    sparse_copy(&image_path, &sparse_path);

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},factory:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&sparse_path)
        .arg("--input-bmap")
        .arg(&input_bmap_path)
        .arg("-b")
        .assert()
        .success();

    let meta = sparse_path.metadata().expect("image metadata");
    assert!(meta.blocks() * STAT_BLOCK_SIZE < meta.len());
    let bmap = read_bmap(&bmap_path);
    let input_bmap = read_bmap(&input_bmap_path);
    assert!(!maps_whole_image(&bmap));
    assert!(maps_all_ranges_of(&bmap, &input_bmap));

    // every data block and every block of the input bmap must replace the old content
    let image = fs::read(&sparse_path).expect("read image");
    let device = bmap_flash(&image, &bmap);
    let block_size = usize::try_from(input_bmap.block_size).expect("block size");
    let blocks = image.chunks(block_size).zip(device.chunks(block_size));
    for (block, (image_block, device_block)) in (0u64..).zip(blocks) {
        let needed = image_block.iter().any(|b| *b != 0)
            || input_bmap.ranges.iter().any(|r| r.contains(&block));
        assert!(
            !needed || image_block == device_block,
            "block {block} keeps old content"
        );
    }
}

#[test]
fn check_file_copy_maps_zero_blocks_of_copied_files() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let input_bmap_path = tr.to_pathbuf("testfiles/image.wic.bmap");
    let sparse_path = image_path.with_file_name("sparse.wic");
    let bmap_path = bmap_path(&sparse_path);
    let device_path = tr.pathbuf().join("device.wic");
    let in_file = tr.pathbuf().join("zero-blocks.bin");
    let in_content = [
        vec![0x11; ZERO_BLOCKS_FILE_EDGE],
        vec![0; ZERO_BLOCKS_FILE_ZEROS],
        vec![0x22; ZERO_BLOCKS_FILE_EDGE],
    ]
    .concat();
    fs::write(&in_file, &in_content).expect("write input file");
    sparse_copy(&image_path, &sparse_path);

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},factory:/my-file", in_file.to_string_lossy()))
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&sparse_path)
        .arg("--input-bmap")
        .arg(&input_bmap_path)
        .arg("-b")
        .assert()
        .success();

    let image = fs::read(&sparse_path).expect("read image");
    fs::write(&device_path, bmap_flash(&image, &read_bmap(&bmap_path))).expect("write device");

    for partition in ["factory", "boot"] {
        let out_file = tr.pathbuf().join(format!("{partition}-out.bin"));
        Command::cargo_bin("omnect-cli")
            .expect("omnect-cli binary")
            .arg("file")
            .arg("copy-from-image")
            .arg("-f")
            .arg(format!(
                "{partition}:/my-file,{}",
                out_file.to_string_lossy()
            ))
            .arg("-i")
            .arg(&device_path)
            .assert()
            .success();
        assert!(
            fs::read(&out_file).expect("read output file") == in_content,
            "{partition}: file differs after a bmap flash"
        );
    }
}

#[test]
fn check_input_bmap_of_another_image() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let bmap_path = bmap_path(&image_path);
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let out_file = tr.pathbuf().join("hosts");
    fs::write(&bmap_path, OTHER_IMAGE_BMAP).expect("write bmap");
    let image_hash = Testrunner::file_hash(&image_path);

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .arg("-b")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot use input bmap"))
        .stderr(predicate::str::contains(OTHER_IMAGE_SIZE_ERROR));
    assert_eq!(Testrunner::file_hash(&image_path), image_hash);

    // reading from the image does not use the bmap
    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!("rootA:/etc/hosts,{}", out_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .assert()
        .success();
}

#[test]
fn check_file_copy_without_output_bmap_keeps_image_sparse() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let sparse_path = image_path.with_file_name("sparse.wic");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    sparse_copy(&image_path, &sparse_path);

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},factory:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&sparse_path)
        .assert()
        .success();

    let meta = sparse_path.metadata().expect("image metadata");
    assert!(
        meta.blocks() * STAT_BLOCK_SIZE < meta.len(),
        "image is not sparse"
    );
}

#[test]
fn check_output_image_that_is_no_regular_file() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic.xz");
    let written_image_path = image_path.with_extension("");
    let bmap_path = bmap_path(&written_image_path);
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    create_dir_all(&written_image_path).expect("create directory");

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .arg("-b")
        .arg("-o")
        .arg(&written_image_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a regular file"));
    assert!(written_image_path.is_dir(), "the directory was replaced");
    assert!(
        !bmap_path.exists(),
        "bmap written for an image that was not written back"
    );
}

#[test]
fn check_input_bmap_requires_output_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let bmap_path = tr.to_pathbuf("testfiles/image.wic.bmap");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let image_hash = Testrunner::file_hash(&image_path);

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .arg("--input-bmap")
        .arg(&bmap_path)
        .assert()
        .failure();
    assert_eq!(Testrunner::file_hash(&image_path), image_hash);
}

#[test]
fn check_input_bmap_of_another_image_with_the_same_size() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let bmap_path = tr.to_pathbuf("testfiles/image.wic.bmap");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let bmap = read_bmap(&bmap_path);
    // another build: one byte differs in a block the bmap maps
    let mapped_byte = usize::try_from(bmap.ranges[0].start() * bmap.block_size).expect("offset");
    let mut image = fs::read(&image_path).expect("read image");
    image[mapped_byte] = !image[mapped_byte];
    fs::write(&image_path, image).expect("write image");
    let image_hash = Testrunner::file_hash(&image_path);

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .arg("-b")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot use input bmap"))
        .stderr(predicate::str::contains("--input-bmap"));
    assert_eq!(Testrunner::file_hash(&image_path), image_hash);
}

#[test]
fn check_chained_commands_with_input_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_xz_path = tr.to_pathbuf("testfiles/image.wic.xz");
    let image_xz_hash = Testrunner::file_hash(&image_xz_path);
    let input_bmap_path = tr.to_pathbuf("testfiles/image.wic.bmap");
    let input_bmap_hash = Testrunner::file_hash(&input_bmap_path);
    let device_path = tr.pathbuf().join("device.wic");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let copy_to_image = |image: &Path, args: &[&OsStr]| {
        Command::cargo_bin("omnect-cli")
            .expect("omnect-cli binary")
            .arg("file")
            .arg("copy-to-image")
            .arg("-f")
            .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
            .arg("-i")
            .arg(image)
            .args(args)
            .assert()
    };

    // image.wic would be written next to image.wic.bmap without updating it
    copy_to_image(&image_xz_path, &[])
        .failure()
        .stderr(predicate::str::contains("pass -b to write a matching bmap"));
    // bmaptool would use image.wic.bmap for the changed image.wic.xz
    copy_to_image(&image_xz_path, &["-p".as_ref(), "xz".as_ref()])
        .failure()
        .stderr(predicate::str::contains("pass -b to write a matching bmap"));
    // the bmap of image.wic would replace the bmap of image.wic.xz
    copy_to_image(&image_xz_path, &["-b".as_ref()])
        .failure()
        .stderr(predicate::str::contains("--output-image"));
    copy_to_image(&image_xz_path, &["-b".as_ref(), "--no-input-bmap".as_ref()])
        .failure()
        .stderr(predicate::str::contains("--output-image"));

    copy_to_image(
        &image_xz_path,
        &["-b".as_ref(), "-o".as_ref(), device_path.as_os_str()],
    )
    .success();
    copy_to_image(&device_path, &["-b".as_ref()]).success();
    copy_to_image(&device_path, &[])
        .failure()
        .stderr(predicate::str::contains("pass -b to write a matching bmap"));

    assert_eq!(Testrunner::file_hash(&image_xz_path), image_xz_hash);
    assert_eq!(Testrunner::file_hash(&input_bmap_path), input_bmap_hash);
    assert!(maps_all_ranges_of(
        &read_bmap(&bmap_path(&device_path)),
        &read_bmap(&input_bmap_path)
    ));
}

#[test]
fn check_derived_output_image_exists() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_xz_path = tr.to_pathbuf("testfiles/image.wic.xz");
    let image_path = image_xz_path.with_extension("");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    fs::write(&image_path, "an older image").expect("write image");
    let copy_to_image = |args: &[&OsStr]| {
        Command::cargo_bin("omnect-cli")
            .expect("omnect-cli binary")
            .arg("file")
            .arg("copy-to-image")
            .arg("-f")
            .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
            .arg("-i")
            .arg(&image_xz_path)
            .args(args)
            .assert()
    };

    copy_to_image(&[])
        .failure()
        .stderr(predicate::str::contains("--output-image"));
    copy_to_image(&[
        "-o".as_ref(),
        image_path.as_os_str(),
        "-p".as_ref(),
        "xz".as_ref(),
    ])
    .failure()
    .stderr(predicate::str::contains(
        "does not end in .xz, the extension of the -p compression; name it e.g. ",
    ));
    // a path given with -o is overwritten
    copy_to_image(&["-o".as_ref(), image_path.as_os_str()]).success();
    assert_ne!(
        fs::read(&image_path).expect("read image"),
        b"an older image"
    );
}

#[test]
fn check_no_input_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let bmap_path = bmap_path(&image_path);
    let other_bmap_path = tr.pathbuf().join("other.bmap");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    fs::write(&bmap_path, OTHER_IMAGE_BMAP).expect("write bmap");
    fs::write(&other_bmap_path, OTHER_IMAGE_BMAP).expect("write bmap");
    let copy_to_image = |args: &[&OsStr]| {
        Command::cargo_bin("omnect-cli")
            .expect("omnect-cli binary")
            .arg("file")
            .arg("copy-to-image")
            .arg("-f")
            .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
            .arg("-i")
            .arg(&image_path)
            .args(args)
            .assert()
    };

    copy_to_image(&["--no-input-bmap".as_ref()]).failure();
    copy_to_image(&[
        "-b".as_ref(),
        "--no-input-bmap".as_ref(),
        "--input-bmap".as_ref(),
        other_bmap_path.as_os_str(),
    ])
    .failure();
    copy_to_image(&["-b".as_ref()])
        .failure()
        .stderr(predicate::str::contains("--no-input-bmap"))
        .stderr(predicate::str::contains(OTHER_IMAGE_SIZE_ERROR));

    copy_to_image(&["-b".as_ref(), "--no-input-bmap".as_ref()]).success();
    assert!(maps_whole_image(&read_bmap(&bmap_path)));
}

#[test]
fn check_bmap_generation_wic() {
    let tr = Testrunner::new(function_name!().split("::").last().unwrap());
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let image_path_copy = PathBuf::from(format!("{}.copy", image_path.to_str().unwrap()));
    let bmap_path = bmap_path(&image_path);
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
    let image_path_wic = tr.pathbuf().join("device.wic");
    let image_path_wic_copy = image_path_wic_xz.with_extension("copy");
    let image_path_bmap = bmap_path(&image_path_wic);
    let input_bmap = read_bmap(&tr.to_pathbuf("testfiles/image.wic.bmap"));
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
        .arg("-o")
        .arg(&image_path_wic)
        .assert();
    assert.success();

    let bmap = read_bmap(&image_path_bmap);
    assert!(!maps_whole_image(&bmap));
    assert!(maps_all_ranges_of(&bmap, &input_bmap));

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
fn check_bmap_generation_wic_xz_without_input_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path_wic_xz = tr.to_pathbuf("testfiles/image.wic.xz");
    // the output image is the uncompressed image next to the input
    let image_path_bmap = bmap_path(&image_path_wic_xz.with_extension(""));
    let in_file = tr.to_pathbuf("testfiles/boot.scr");

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path_wic_xz)
        .arg("-b")
        .assert()
        .success();

    assert!(maps_whole_image(&read_bmap(&image_path_bmap)));
}

#[test]
fn check_bmap_of_compressed_output_keeps_input_bmap() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path = tr.to_pathbuf("testfiles/image.wic");
    let input_bmap_path = tr.to_pathbuf("testfiles/image.wic.bmap");
    let input_bmap_hash = Testrunner::file_hash(&input_bmap_path);
    let image_path_xz = image_path.with_extension("wic.xz");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-to-image")
        .arg("-f")
        .arg(format!("{},boot:/my-file", in_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path)
        .arg("-b")
        .arg("-p")
        .arg("xz")
        .assert()
        .success();

    assert_eq!(
        Testrunner::file_hash(&input_bmap_path),
        input_bmap_hash,
        "bmap of the unchanged input image was replaced"
    );
    // bmaptool finds <image>.bmap without --bmap and checks the image against it
    Command::new("bmaptool")
        .arg("copy")
        .arg(&image_path_xz)
        .arg(image_path.with_extension("copy"))
        .assert()
        .success()
        .stderr(predicate::str::contains("image.wic.xz.bmap"));
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

#[test]
fn check_copy_from_image_writes_no_image() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    let image_path_wic_xz = tr.to_pathbuf("testfiles/image.wic.xz");
    let image_hash = Testrunner::file_hash(&image_path_wic_xz);
    let out_file = tr.pathbuf().join("hosts");

    Command::cargo_bin("omnect-cli")
        .expect("omnect-cli binary")
        .arg("file")
        .arg("copy-from-image")
        .arg("-f")
        .arg(format!("rootA:/etc/hosts,{}", out_file.to_string_lossy()))
        .arg("-i")
        .arg(&image_path_wic_xz)
        .assert()
        .success();

    assert!(out_file.exists());
    assert!(!image_path_wic_xz.with_extension("").exists());
    assert_eq!(Testrunner::file_hash(&image_path_wic_xz), image_hash);
}

#[test]
fn check_partition_files_stay_out_of_the_image_dir() {
    let tr = Testrunner::new(function_name!().split("::").last().expect("test name"));
    // the name of the file that holds partition 1
    let image_path = tr.pathbuf().join("1.img");
    fs::rename(tr.to_pathbuf("testfiles/image.wic"), &image_path).expect("rename image");
    let in_file = tr.to_pathbuf("testfiles/boot.scr");
    let out_file = tr.pathbuf().join("my-file");
    let file_image = |command: &str, files: String| {
        Command::cargo_bin("omnect-cli")
            .expect("omnect-cli binary")
            .arg("file")
            .arg(command)
            .arg("-f")
            .arg(files)
            .arg("-i")
            .arg(&image_path)
            .assert()
            .success();
    };
    let dir_entries = || {
        let mut entries = fs::read_dir(tr.pathbuf())
            .expect("read test dir")
            .map(|entry| entry.expect("dir entry").file_name())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    };

    file_image(
        "copy-to-image",
        format!("{},boot:/my-file", in_file.to_string_lossy()),
    );
    let entries = dir_entries();
    file_image(
        "copy-from-image",
        format!("boot:/my-file,{}", out_file.to_string_lossy()),
    );

    assert_eq!(
        fs::read(&out_file).expect("read out file"),
        fs::read(&in_file).expect("read in file")
    );
    let mut expected = entries;
    expected.push(out_file.file_name().expect("file name").to_owned());
    expected.sort();
    assert_eq!(dir_entries(), expected);
}

#[tokio::test]
async fn check_ssh_tunnel_setup() {
    let tr = Testrunner::new("check_ssh_tunnel_setup");

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

    let config = ssh::Config::new(server.base_url(), Some(tr.pathbuf()), None, None).unwrap();

    ssh::ssh_create_tunnel("test_device", "test_user", config, mock_access_token)
        .await
        .unwrap();

    assert!(
        tr.pathbuf()
            .join("test_device_config")
            .try_exists()
            .is_ok_and(|exists| exists)
    );
    assert!(
        tr.pathbuf()
            .join("test_device_id_ed25519")
            .try_exists()
            .is_ok_and(|exists| exists)
    );
    assert!(
        tr.pathbuf()
            .join("test_device_id_ed25519.pub")
            .try_exists()
            .is_ok_and(|exists| exists)
    );
    assert!(
        tr.pathbuf()
            .join("test_device_bastion-cert.pub")
            .try_exists()
            .is_ok_and(|exists| exists)
    );
    assert!(
        tr.pathbuf()
            .join("test_device_device-cert.pub")
            .try_exists()
            .is_ok_and(|exists| exists)
    );

    let ssh_config = std::fs::read_to_string(tr.pathbuf().join("test_device_config")).unwrap();
    let expected_config = format!(
        r#"Host bastion
	User bastion_user
	Hostname 132.23.0.1
	Port 22
	IdentityFile {}/test_device_id_ed25519
	CertificateFile {}/test_device_bastion-cert.pub
	ProxyCommand none

Host test_device
	User test_user
	IdentityFile {}/test_device_id_ed25519
	CertificateFile {}/test_device_device-cert.pub
	ProxyCommand ssh -F {}/test_device_config bastion
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
async fn check_multi_ssh_tunnel_setup() {
    let tr = Testrunner::new("check_multi_ssh_tunnel_setup");

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

    ssh::ssh_create_tunnel(
        "test_device_a",
        "test_user",
        ssh::Config::new(server.base_url(), Some(tr.pathbuf()), None, None).unwrap(),
        mock_access_token.clone(),
    )
    .await
    .unwrap();

    ssh::ssh_create_tunnel(
        "test_device_b",
        "test_user",
        ssh::Config::new(server.base_url(), Some(tr.pathbuf()), None, None).unwrap(),
        mock_access_token,
    )
    .await
    .unwrap();

    for device in ["test_device_a", "test_device_b"] {
        assert!(
            tr.pathbuf()
                .join(format!("{device}_config"))
                .try_exists()
                .is_ok_and(|exists| exists)
        );
        assert!(
            tr.pathbuf()
                .join(format!("{device}_id_ed25519"))
                .try_exists()
                .is_ok_and(|exists| exists)
        );
        assert!(
            tr.pathbuf()
                .join(format!("{device}_id_ed25519.pub"))
                .try_exists()
                .is_ok_and(|exists| exists)
        );
        assert!(
            tr.pathbuf()
                .join(format!("{device}_bastion-cert.pub"))
                .try_exists()
                .is_ok_and(|exists| exists)
        );
        assert!(
            tr.pathbuf()
                .join(format!("{device}_device-cert.pub"))
                .try_exists()
                .is_ok_and(|exists| exists)
        );

        let ssh_config =
            std::fs::read_to_string(tr.pathbuf().join(format!("{device}_config"))).unwrap();
        let expected_config = format!(
            r#"Host bastion
	User bastion_user
	Hostname 132.23.0.1
	Port 22
	IdentityFile {}/{device}_id_ed25519
	CertificateFile {}/{device}_bastion-cert.pub
	ProxyCommand none

Host {device}
	User test_user
	IdentityFile {}/{device}_id_ed25519
	CertificateFile {}/{device}_device-cert.pub
	ProxyCommand ssh -F {}/{device}_config bastion
"#,
            tr.pathbuf().to_string_lossy(),
            tr.pathbuf().to_string_lossy(),
            tr.pathbuf().to_string_lossy(),
            tr.pathbuf().to_string_lossy(),
            tr.pathbuf().to_string_lossy()
        );

        assert_eq!(ssh_config, expected_config);
    }
}

// currently disabled as we have no way to test this in our pipeline were we
// don't have docker installed
#[ignore]
#[test]
fn check_docker_inject_image_success() {
    let image = std::env::var("TEST_DOCKER_IMAGE").unwrap_or("busybox".to_string());

    let tr = Testrunner::new(function_name!().split("::").last().unwrap());

    let image_path = tr.to_pathbuf("testfiles/image.wic");

    let mut docker_inject_image_config = Command::cargo_bin("omnect-cli").unwrap();
    let assert = docker_inject_image_config
        .arg("docker")
        .arg("inject")
        .args(["--docker-image", &image])
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
