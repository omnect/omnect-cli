use std::{env, io::Write, fs};

const DEFAULT_DOCKER_REG_NAME: &'static str = "icsdm.azurecr.io";
const ICS_DM_CLI_DOCKER_REG_NAME_FILE: &'static str = "gen/ICS_DM_CLI_DOCKER_REG_NAME.txt";

fn main() {
    let mut fh = fs::File::create(&ICS_DM_CLI_DOCKER_REG_NAME_FILE).unwrap();
    if let Some(reg_name) = env::var_os("ICS_DM_CLI_DOCKER_REG_NAME")
    {
        write!(fh, r#""{}" //auto-generated"#, reg_name.to_str().unwrap()).unwrap();
    }
    else
    {
        write!(fh, r#""{}" //auto-generated"#, DEFAULT_DOCKER_REG_NAME).unwrap();
    }

    // when to rebuild:
    println!("cargo:rerun-if-env-changed=ICS_DM_CLI_DOCKER_REG_NAME");
    println!("cargo:rerun-if-changed=src/build.rs");
}
