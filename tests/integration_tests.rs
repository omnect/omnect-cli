use ics_dm_cli::file;
use std::path::PathBuf;
use std::fs::File;

mod common;


#[test]

fn check_file_exists() {
    common::setup();

    let path_ok = PathBuf::from(r"./tmp/tesfile");
    let path_err = PathBuf::from(r"not_existing_file");

    File::create(path_ok.clone()).unwrap();

    assert_eq!(true, file::file_exits(&path_ok).is_ok());
    assert_eq!(false, file::file_exits(&path_err).is_ok());

    common::cleanup();
}