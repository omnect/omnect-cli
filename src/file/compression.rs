use anyhow::{Context, Result};
use filemagic::Magic;
use log::debug;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone, Debug, EnumIter)]
#[allow(non_camel_case_types)]
pub enum Compression {
    xz,
    bzip2,
    gzip,
}

impl FromStr for Compression {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Compression, Self::Err> {
        match input {
            "xz" => Ok(Compression::xz),
            "bzip2" => Ok(Compression::bzip2),
            "gzip" => Ok(Compression::gzip),
            _ => anyhow::bail!("unknown compression: use either xz, bzip2 or gzip"),
        }
    }
}

impl Compression {
    fn compress(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut enc: Box<dyn std::io::Write> = match &self {
            Compression::bzip2 => Box::new(bzip2::write::BzEncoder::new(
                destination,
                bzip2::Compression::best(),
            )),
            Compression::gzip => Box::new(flate2::write::GzEncoder::new(
                destination,
                flate2::Compression::best(),
            )),
            Compression::xz => {
                let level = env::var("XZ_COMPRESSION_LEVEL")
                    .unwrap_or_else(|_| "9".to_string())
                    .parse()
                    .unwrap_or(9);

                let level = if (0..=9).contains(&level) { level } else { 9 };
                let stream = xz2::stream::MtStreamBuilder::new()
                    .threads(num_cpus::get() as u32)
                    .preset(level)
                    .encoder()?;
                Box::new(xz2::write::XzEncoder::new_stream(destination, stream))
            }
        };

        let bytes_written = std::io::copy(source, &mut enc)?;
        enc.flush()?;
        Ok(bytes_written)
    }

    fn decompress(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut dec: Box<dyn std::io::Write> = match &self {
            Compression::bzip2 => Box::new(bzip2::write::BzDecoder::new(destination)),
            Compression::gzip => Box::new(flate2::write::GzDecoder::new(destination)),
            Compression::xz => Box::new(xz2::write::XzDecoder::new(destination)),
        };

        let bytes_written = std::io::copy(source, &mut dec)?;
        dec.write_all(&[])?;
        dec.flush()?;
        Ok(bytes_written)
    }

    fn marker(&self) -> &'static str {
        match &self {
            Compression::bzip2 => "bzip2 compressed data",
            Compression::gzip => "gzip compressed data",
            Compression::xz => "XZ compressed data",
        }
    }

    fn extension(&self) -> &'static str {
        match &self {
            Compression::bzip2 => "bzip2",
            Compression::gzip => "gzip",
            Compression::xz => "xz",
        }
    }

    pub fn from_file(image_file_name: &PathBuf) -> Result<Option<Compression>> {
        let detector = Magic::open(Default::default())
            .context("image::compression: failed to open libmagic")?;

        detector
            .load::<String>(&[])
            .context("image::compression: failed to load libmagic")?;

        let magic = detector
            .file(image_file_name)
            .context("image::compression: failed to open image")?;

        for c in Compression::iter() {
            if magic.contains(c.marker()) {
                return Ok(Some(c));
            }
        }

        Ok(None)
    }
}

pub fn decompress(image_file_name: &PathBuf, compression: &Compression) -> Result<PathBuf> {
    let mut new_image_file = PathBuf::from(image_file_name);

    if let Some(extension) = new_image_file.extension() {
        if extension == compression.extension() {
            new_image_file.set_extension("");
        }
    }

    let mut destination = File::create(&new_image_file)?;
    let mut source = File::open(image_file_name)?;
    let bytes_written = compression.decompress(&mut source, &mut destination)?;
    debug!("image::decompress: copied {} bytes.", bytes_written);
    Ok(new_image_file)
}

pub fn compress(image_file_name: &PathBuf, compression: &Compression) -> Result<PathBuf> {
    let new_image_file = PathBuf::from(format!(
        "{}.{}",
        image_file_name.to_str().unwrap(),
        compression.extension()
    ));
    let mut destination = File::create(&new_image_file)?;
    let mut source = File::open(image_file_name)?;
    let bytes_written = compression.compress(&mut source, &mut destination)?;
    debug!("image::compress: copied {} bytes.", bytes_written);
    Ok(new_image_file)
}
