use data_encoding::HEXUPPER;
use env_logger::{Builder, Env};
use ring::digest::{Context, SHA256};
use std::fs::{File, copy, create_dir_all, remove_dir_all};
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::LazyLock;

const TMPDIR_FORMAT_STR: &str = "/tmp/omnect-cli-integration-tests/";

static LOG: LazyLock<()> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        Builder::from_env(Env::default().default_filter_or("debug")).init()
    } else {
        Builder::from_env(Env::default().default_filter_or("info")).init()
    }
});

pub struct Testrunner {
    dirpath: std::string::String,
}

impl Testrunner {
    pub fn new(prefix: &str) -> Testrunner {
        LazyLock::force(&LOG);
        let dirpath = format!("{}{}", TMPDIR_FORMAT_STR, prefix);
        create_dir_all(&dirpath).unwrap();
        Testrunner { dirpath }
    }

    pub fn pathbuf(&self) -> PathBuf {
        PathBuf::from(&self.dirpath)
    }

    pub fn to_pathbuf(&self, file: &str) -> PathBuf {
        let destfile = String::from(file);
        let destfile = destfile.split('/').next_back().unwrap();
        let path = PathBuf::from(format!("{}/{}", self.dirpath, destfile));

        copy(file, &path).unwrap();
        path
    }
    pub fn file_hash(path: &PathBuf) -> String {
        let mut context = Context::new(&SHA256);
        let mut buffer = [0; 1024];
        let input = File::open(path).unwrap();
        let mut reader = BufReader::new(input);

        loop {
            let count = reader.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            context.update(&buffer[..count]);
        }

        HEXUPPER.encode(context.finish().as_ref())
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
