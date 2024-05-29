use std::path::Path;

use crate::file::functions::read_file_from_image;
use crate::file::functions::Partition;
use anyhow::{Context, Result};
use regex::Regex;

// NOTE (2024-05-29 Tobias Langer): /etc/os-release is a symlink in our yocto
// builds. The e2tools-suite cannot handle symlinks so we use its target
// directly.
const OS_RELEASE_PATH: &str = "/usr/lib/os-release";
const OS_RELEASE_PARTITION: Partition = Partition::rootA;

lazy_static::lazy_static! {
    pub static ref ARCH_REGEX: Regex = {
        Regex::new(r#"OMNECT_TARGET_ARCH="(?<arch>.*)""#).unwrap()
    };
}

pub enum Architecture {
    Arm64,
}

impl TryInto<Architecture> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Architecture> {
        let arch = match self {
            "aarch64" => Architecture::Arm64,
            _ => {
                anyhow::bail!("unknown architecture: {self}")
            }
        };

        Ok(arch)
    }
}

pub fn image_arch(image: impl AsRef<Path>) -> Result<Architecture> {
    let os_release_info = read_file_from_image(OS_RELEASE_PATH, OS_RELEASE_PARTITION, image)
        .context("image_arch: could not read os-release info")?;

    let arch = ARCH_REGEX
        .captures(&os_release_info)
        .ok_or(anyhow::anyhow!(
            "image_arch: os-release does not contain architecture information"
        ))?;

    arch["arch"]
        .try_into()
        .context(format!("Unsupported architecture type: {}", &arch["arch"]))
}
