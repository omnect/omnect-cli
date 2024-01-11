use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn validate_config(device_update_conf_file: &Path) -> Result<()> {
    let file = File::open(device_update_conf_file).context(format!(
        "validate_du_config: failed to open {device_update_conf_file:?}"
    ))?;
    serde_json::from_reader::<_, serde_json::Value>(BufReader::new(file))
        .context("validate_du_config: read config_file")?;

    // ToDo: add further checks

    Ok(())
}
