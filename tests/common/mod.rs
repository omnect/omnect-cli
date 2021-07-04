use std::fs;


pub fn setup() {
    fs::create_dir("./tmp").unwrap();

}

pub fn cleanup() {
    fs::remove_dir_all("./tmp").unwrap();
}