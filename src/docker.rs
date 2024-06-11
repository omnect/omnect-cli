use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::image::Architecture;

impl From<Architecture> for &str {
    fn from(arch: Architecture) -> &'static str {
        match arch {
            Architecture::ARM32 => "linux/arm/v7",
            Architecture::ARM64 => "linux/arm64",
            Architecture::x86_64 => "linux/amd64",
        }
    }
}

use crate::file::compression::Compression;
use std::fs::File;
use std::os::fd::AsFd;
use std::process::{Command, Stdio};

const COMPRESSION_LEVEL: u32 = 9;

pub fn pull_image(name: impl AsRef<str>, arch: Architecture) -> Result<PathBuf> {
    let cmd_out = Command::new("docker")
        .args(["pull"])
        .args(["--platform", arch.into()])
        .arg(name.as_ref())
        .output()
        .context("pull_docker_image: could not run \"docker pull\" command")?;

    if !cmd_out.status.success() {
        let cmd_out = std::str::from_utf8(&cmd_out.stderr).unwrap();
        anyhow::bail!("Could not pull docker image: {cmd_out}");
    }

    let mut child = Command::new("docker")
        .args(["save"])
        .arg(name.as_ref())
        .stdout(Stdio::piped())
        .spawn()
        .context("pull_docker_image: could not run \"docker save\" command")?;

    let stdout = child.stdout.take().unwrap();
    let mut image_file = File::from(stdout.as_fd().try_clone_to_owned()?);

    let out_path = std::path::PathBuf::from(format!("{}.tar.xz", name.as_ref()));
    let mut out_file = std::fs::File::options()
        .create_new(true)
        .write(true)
        .open(out_path.clone())
        .context("pull_docker_image: could not create output file")?;

    let xz = Compression::xz {
        compression_level: COMPRESSION_LEVEL,
    };
    xz.compress(&mut image_file, &mut out_file)?;

    let error_code = child.wait()?;

    if !error_code.success() {
        let cmd_out = std::str::from_utf8(&cmd_out.stderr).unwrap();
        anyhow::bail!("Could not save docker image: {cmd_out}");
    }

    Ok(out_path)
}
