use std::fs;
extern crate fs_extra;
use fs_extra::dir::copy;
use fs_extra::dir::CopyOptions;

pub fn setup() {
    let options = CopyOptions::new(); 
    copy("testfiles", "tests", &options).unwrap();
}

pub fn cleanup() {
    fs::remove_dir_all("tests/testfiles").unwrap();
}