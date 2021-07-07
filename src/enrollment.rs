use crate::docker;
use crate::file;
use std::io::{Error, ErrorKind};

pub fn config(enrollment_config_file: std::path::PathBuf, provisioning_config_file: std::path::PathBuf, image_file: std::path::PathBuf ) -> Result<(),Error> {
    file::file_exists(&enrollment_config_file)?;
    file::file_exists(&provisioning_config_file)?;
    file::file_exists(&image_file)?;

    /*
        todo some content verification of config_file and image_file?
        e.g. image_file currently should be an uncompressed wic file
    */

    docker::set_wifi_config(enrollment_config_file.to_str().unwrap(), image_file.to_str().unwrap())
}

pub fn info(image_file: std::path::PathBuf) -> Result<(),Error> {
    file::file_exists(&image_file)?;

    Err(Error::new(ErrorKind::Other, "Not implemented"))
}
