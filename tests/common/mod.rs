use std::fs;


pub fn setup() {
    fs::create_dir("./tmp")?;

}

pub fn cleanup() {
    fs::remove_dir("./tmp")?;
}