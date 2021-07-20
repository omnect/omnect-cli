use std::fs::{create_dir,remove_dir_all};
use fs_extra::dir::copy;
use fs_extra::dir::CopyOptions;

pub fn setup(prefix: &str) {
    create_dir(prefix).unwrap();
    copy("testfiles", prefix, &CopyOptions{
        overwrite: true,
        ..Default::default()}).unwrap_or_else(|err| {
            // ignore all errors if dir cannot be deleted
            println!("Problem copy: {}", err);
            1
        });

}

pub fn cleanup(prefix: &str) {
    // place your cleanup code here
    remove_dir_all(prefix).unwrap_or_else(|err| {
        // ignore all errors if dir cannot be deleted
        println!("Problem remove_dir_all: {}", err);
    });
}
