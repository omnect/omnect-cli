fn main() {
    // when to rebuild:
    println!("cargo:rerun-if-env-changed=ICS_DM_CLI_DOCKER_REG_NAME");
    println!("cargo:rerun-if-changed=src/build.rs");
}
