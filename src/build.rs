fn main() {
    // when to rebuild:
    println!("cargo:rerun-if-env-changed=DEFAULT_DOCKER_REG_NAME");
    println!("cargo:rerun-if-changed=src/build.rs");
}
