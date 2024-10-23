use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::file::compression::Compression;
use crate::image::Architecture;
use std::fs::{self, File};
use std::os::fd::AsFd;
use std::process::{Command, Stdio};

impl From<Architecture> for &str {
    fn from(arch: Architecture) -> &'static str {
        match arch {
            Architecture::ARM32 => "linux/arm/v7",
            Architecture::ARM64 => "linux/arm64",
            Architecture::x86_64 => "linux/amd64",
        }
    }
}

pub fn pull_image(name: impl AsRef<str>, arch: Architecture) -> Result<PathBuf> {
    if let Ok("true") | Ok("1") = std::env::var("CONTAINERIZED").as_deref() {
        anyhow::bail!("pull_docker_image: not supported in containerized environments.");
    }

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

    let out_path = std::path::PathBuf::from("./image.tar.gz");
    let mut out_file = std::fs::File::options()
        .create_new(true)
        .write(true)
        .open(out_path.clone())
        .context(format!(
            "pull_docker_image: could not create output file {}",
            fs::canonicalize(&out_path).unwrap().to_string_lossy(),
        ))?;

    Compression::gzip.compress(&mut image_file, &mut out_file)?;

    let error_code = child.wait()?;

    if !error_code.success() {
        let cmd_out = std::str::from_utf8(&cmd_out.stderr).unwrap();
        anyhow::bail!("Could not save docker image: {cmd_out}");
    }

    Ok(out_path)
}
