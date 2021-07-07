use std::io::{Error, ErrorKind};

pub fn file_exists(file: &std::path::PathBuf) -> Result<(),Error> {
    std::fs::metadata(&file)
    .map_err(|e| {Error::new(e.kind(), e.to_string() + ": " + file.to_str().unwrap())})?
    .is_file()
    .then(|| ())
    .ok_or(Error::new(ErrorKind::InvalidInput, file.to_str().unwrap().to_owned() + &" is not a file path"))?;

    Ok(())
}
