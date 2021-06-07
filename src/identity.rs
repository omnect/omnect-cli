use crate::file;
use std::io::{Error, ErrorKind};

pub fn config(config_file: std::path::PathBuf, image_file: std::path::PathBuf ) -> Result<(),Error> {
    file::file_exits(&config_file)?;
    file::file_exits(&image_file)?;

    Err(Error::new(ErrorKind::Other, "Not implemented"))
}

pub fn info(image_file: std::path::PathBuf) -> Result<(),Error> {
    file::file_exits(&image_file)?;

    Err(Error::new(ErrorKind::Other, "Not implemented"))
}
