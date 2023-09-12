use env_logger::{Builder, Env};
use std::fs::copy;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::PathBuf;

const TMPDIR_FORMAT_STR: &str = "/tmp/omnect-cli-integration-tests/";

lazy_static! {
    static ref LOG: () = if cfg!(debug_assertions) {
        Builder::from_env(Env::default().default_filter_or("debug,bollard::read=info")).init()
    } else {
        Builder::from_env(Env::default().default_filter_or("info")).init()
    };
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

    pub fn pathbuf(&self) -> PathBuf {
        PathBuf::from(&self.dirpath)
    }

    pub fn to_pathbuf(&self, file: &str) -> PathBuf {
        let destfile = String::from(file);
        let destfile = destfile.split('/').last().unwrap();

        let path = PathBuf::from(format!("{}/{}", self.dirpath, destfile));

        /*
         * unfortunately std::fs::copy can not handle sparse files.
         * we can't test bmaptool file handling in the integration test,
         * because image.wic is not a sparse file after the copy anymore
         *
         * reason: declining of https://github.com/rust-lang/rust/issues/58635
         * possible alternative: https://crates.io/crates/hole-punch
         * possible alternative for unix hosts only: https://github.com/rust-lang/rust/pull/58636/files
         */
        copy(file, &path).unwrap();
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
