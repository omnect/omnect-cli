use std::fs::{create_dir,remove_dir_all};
use fs_extra::dir::copy;
use fs_extra::dir::CopyOptions;

pub struct Testrunner<'a> {
    prefix: &'a str
}

impl<'a> Testrunner<'a> {
    pub fn new(prefix: &str) -> Testrunner {
        create_dir(prefix).unwrap();
        copy("testfiles", prefix, &CopyOptions {
            overwrite: true,
            ..Default::default()}).unwrap_or_else(|err| {
                // ignore all errors if dir cannot be deleted
                println!("Problem copy: {}", err);
                1
            }
        );
        Testrunner { prefix }
    }
}

impl<'a> Drop for Testrunner<'a> {
    fn drop(&mut self) {
        // place your cleanup code here
        remove_dir_all(self.prefix).unwrap_or_else(|err| {
            // ignore all errors if dir cannot be deleted
            println!("Problem remove_dir_all: {}", err);
        });
    }
}