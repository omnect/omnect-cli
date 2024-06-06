use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::image::Architecture;

impl From<Architecture> for &str {
    fn from(arch: Architecture) -> &'static str {
        match arch {
            Architecture::Arm64 => "linux/arm64",
        }
    }
}

#[cfg(not(feature = "mock"))]
mod inner {
    use super::*;

    use std::fs::File;
    use std::os::fd::AsFd;
    use std::process::{Command, Stdio};

    const COMPRESSION_LEVEL: u32 = 9;

    fn compress_image(source: File, destination: File) -> Result<File> {
        let mut source = std::io::BufReader::new(source);

        let destination = std::io::BufWriter::new(destination);

        let stream_builder = xz2::stream::MtStreamBuilder::new()
            .threads(num_cpus::get() as u32)
            .preset(COMPRESSION_LEVEL)
            .encoder()
            .context("pull_docker_image: could not create compression stream")?;

        let mut stream = xz2::write::XzEncoder::new_stream(destination, stream_builder);

        std::io::copy(&mut source, &mut stream)
            .context("compress_docker_image: could not compress docker file")?;

        let writer = stream.finish()?;
        Ok(writer.into_inner()?)
    }

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
        let image_file = File::from(stdout.as_fd().try_clone_to_owned()?);

        let out_path = std::path::PathBuf::from(format!("{}.tar.xz", name.as_ref()));
        let out_file = std::fs::File::options()
            .create_new(true)
            .write(true)
            .open(out_path.clone())
            .context("pull_docker_image: could not create output file")?;

        compress_image(image_file, out_file)?;

        let error_code = child.wait()?;

        if !error_code.success() {
            let cmd_out = std::str::from_utf8(&cmd_out.stderr).unwrap();
            anyhow::bail!("Could not save docker image: {cmd_out}");
        }

        Ok(out_path)
    }
}

#[cfg(feature = "mock")]
mod inner {
    use super::*;

    use std::io::Write;

    pub fn pull_image(name: impl AsRef<str>, _arch: Architecture) -> Result<PathBuf> {
        let out_path = std::path::PathBuf::from(format!("{}.tar.gz", name.as_ref()));
        let mut out_file = std::fs::File::options()
            .create_new(true)
            .write(true)
            .open(out_path.clone())
            .context("pull_docker_image: could not create output file")?;

        out_file.write_all(b"some test data")?;
        Ok(out_path)
    }
}

pub use inner::pull_image;
