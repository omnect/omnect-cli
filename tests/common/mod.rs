use env_logger::{Builder, Env};
use std::fs::copy;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::PathBuf;

const TMPDIR_FORMAT_STR: &'static str = "/tmp/ics-dm-cli-integration-tests/";
const TESTDIR_FORMAT_STR: &'static str = "testfiles/";

lazy_static! {
    static ref LOG: () =
        Builder::from_env(Env::default().default_filter_or("debug,bollard::read=info")).init();
}

pub struct Testrunner {
    dirpath: std::string::String,
}

impl Testrunner {
    pub fn new(prefix: &str) -> Testrunner {
        lazy_static::initialize(&LOG);
        let dirpath = format!("{}{}", TMPDIR_FORMAT_STR, prefix);
        create_dir_all(&dirpath).unwrap();
        Testrunner { dirpath }
    }

    pub fn to_pathbuf(&self, file: &str) -> PathBuf {
        let path = PathBuf::from(format!("{}/{}", self.dirpath, file));
        copy(format!("{}{}", TESTDIR_FORMAT_STR, file), &path).unwrap();
        path
    }
}

impl Drop for Testrunner {
    fn drop(&mut self) {
        // place your cleanup code here
        remove_dir_all(&self.dirpath).unwrap_or_else(|err| {
            // ignore all errors if dir cannot be deleted
            println!("Problem remove_dir_all: {}", err);
        });
    }
}
