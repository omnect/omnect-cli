extern crate path_absolutize;
use ics_dm_cli::wifi;
use ics_dm_cli::identity;
use ics_dm_cli::enrollment;
use std::path::{Path, PathBuf};
use path_absolutize::*;

mod common;


#[test]
fn check_set_wifi_config() {
    common::setup();

    let config_file_path = PathBuf::from(Path::new(r"tests/testfiles/wpa_supplicant.conf").absolutize().unwrap());
    let image_path = PathBuf::from(Path::new(r"tests/testfiles/image.wic").absolutize().unwrap());

    assert_eq!(true, wifi::config(config_file_path, image_path).is_ok());

    common::cleanup();

}

#[test]
fn check_set_enrollment_config() {
    
    common::setup();

    let enrollment_config_file_path = PathBuf::from(Path::new(r"tests/testfiles/enrollment_static.conf").absolutize().unwrap());
    let provisioning_config_file_path = PathBuf::from(Path::new(r"tests/testfiles/provisioning_static.conf").absolutize().unwrap());
    let image_path = PathBuf::from(Path::new(r"tests/testfiles/image.wic").absolutize().unwrap());

    assert_eq!(true, enrollment::config(enrollment_config_file_path, provisioning_config_file_path, image_path).is_ok());

    common::cleanup();
    /**/
}

#[test]
fn check_set_identity() {
    /*
    common::setup();

    let image_path = PathBuf::from(r"tests/testfiles/image.wic");
    assert_eq!(true, identity::info(image_path).is_err());

    common::cleanup();
    */
}