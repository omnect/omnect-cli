use std::fs::{create_dir_all,remove_dir_all};
use fs_extra::dir::copy;
use fs_extra::dir::CopyOptions;


const TMPDIR_FORMAT_STR: &'static str = "/tmp/ics-dm-cli-integration-tests/";

pub struct Testrunner<> {
    dirpath: std::string::String,
}

impl<> Testrunner<> {
    pub fn new(prefix: &str) -> Testrunner {
        let dirpath = format!("{}{}",TMPDIR_FORMAT_STR, prefix);
        create_dir_all(&dirpath).unwrap();
        copy("testfiles", &dirpath, &CopyOptions {
            overwrite: true,
            ..Default::default()
        }).unwrap_or_else(|err| {
            // ignore all errors if dir cannot be deleted
            println!("Problem copy: {}", err);
            1
        });

        Testrunner { dirpath }
    }
}

impl<> Drop for Testrunner<> {
    fn drop(&mut self) {
        // place your cleanup code here
        //remove_dir_all(format!("{}{}", TMPDIR_FORMAT_STR, self.prefix)).unwrap_or_else(|err| {
        remove_dir_all(&self.dirpath).unwrap_or_else(|err| {
            // ignore all errors if dir cannot be deleted
            println!("Problem remove_dir_all: {}", err);
        });
    }
}
