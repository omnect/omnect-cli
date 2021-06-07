use std::io::{Error, ErrorKind};

pub fn file_exits(file: &std::path::PathBuf) -> Result<(),Error> {
    match std::fs::metadata(file){
        Ok(m) => {
            if ! m.is_file() {
                return Err(Error::new(ErrorKind::InvalidInput, "file path is not a path: ".to_owned() + file.to_str().unwrap()));
            }
            else {
                Ok(())
            }
        },
        Err(e) => { return Err(Error::new(e.kind(), file.to_str().unwrap()));}
    }
}
