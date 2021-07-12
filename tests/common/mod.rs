use fs_extra::dir::copy;
use fs_extra::dir::CopyOptions;

pub fn setup() {
    copy("testfiles", "tests", &CopyOptions{
        overwrite: true,
        ..Default::default()}).unwrap_or_else(|err| {
            // ignore all errors if dir cannot be deleted
            println!("Problem copy: {}", err);
            1
        });
}

pub fn cleanup() {
    // place your cleanup code here
}