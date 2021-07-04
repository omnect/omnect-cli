use std::fs::remove_dir_all;
extern crate fs_extra;
use fs_extra::dir::copy;
use fs_extra::dir::CopyOptions;

pub fn setup() {
    copy("testfiles", "tests", &CopyOptions{
        overwrite: true,
        ..Default::default()}).unwrap();
}

pub fn cleanup() {
    remove_dir_all("tests/testfiles").unwrap();
}