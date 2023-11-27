use anyhow::{Context, Result};
use filemagic::Magic;
use log::{debug, info};
use std::env;
use std::fs::remove_file;
use std::fs::File;
use std::path::{Path, PathBuf};

trait CompressionGenerator {
    fn compress(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64>;
    fn decompress(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64>;
}

struct XzGenerator;
impl CompressionGenerator for XzGenerator {
    fn compress<'a>(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let stream = xz2::stream::MtStreamBuilder::new()
            .threads(num_cpus::get() as u32)
            .preset(XzGenerator::get_level())
            .encoder()?;
        let mut enc = xz2::write::XzEncoder::new_stream(destination, stream);

        let bytes_written = std::io::copy(source, &mut enc)?;
        enc.finish()?;
        Ok(bytes_written)
    }
    fn decompress<'a>(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut dec = xz2::write::XzDecoder::new(destination);
        let bytes_written = std::io::copy(source, &mut dec)?;
        dec.finish()?;
        Ok(bytes_written)
    }
}

impl XzGenerator {
    fn get_level() -> u32 {
        let range = 0..9;
        let level = env::var("XZ_COMPRESSION_LEVEL")
            .unwrap_or_else(|_| "9".to_string())
            .parse()
            .unwrap_or(9);

        let level = if range.contains(&level) { level } else { 9 };

        debug!("using Xz compression level: {}", level);

        level
    }
}

struct BzGenerator;
impl CompressionGenerator for BzGenerator {
    fn compress<'a>(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut enc = bzip2::write::BzEncoder::new(destination, bzip2::Compression::best());
        let bytes_written = std::io::copy(source, &mut enc)?;
        enc.finish()?;
        Ok(bytes_written)
    }
    fn decompress<'a>(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut dec = bzip2::write::BzDecoder::new(destination);
        let bytes_written = std::io::copy(source, &mut dec)?;
        dec.finish()?;
        Ok(bytes_written)
    }
}

struct GzGenerator;
impl CompressionGenerator for GzGenerator {
    fn compress<'a>(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut enc = flate2::write::GzEncoder::new(destination, flate2::Compression::best());
        let bytes_written = std::io::copy(source, &mut enc)?;
        enc.finish()?;
        Ok(bytes_written)
    }
    fn decompress<'a>(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let mut dec = flate2::write::GzDecoder::new(destination);
        let bytes_written = std::io::copy(source, &mut dec)?;
        dec.finish()?;
        Ok(bytes_written)
    }
}

struct CompressionAlternative {
    marker: &'static str,
    extension: &'static str,
    generator: &'static dyn CompressionGenerator,
}

const COMPRESSION_TABLE: [CompressionAlternative; 3] = [
    CompressionAlternative {
        marker: "XZ compressed data",
        extension: "unxz.tmp",
        generator: &XzGenerator {},
    },
    CompressionAlternative {
        marker: "bzip2 compressed data",
        extension: "unbzip2.tmp",
        generator: &BzGenerator {},
    },
    CompressionAlternative {
        marker: "gzip compressed data",
        extension: "ungzip.tmp",
        generator: &GzGenerator {},
    },
];

pub fn image_action(
    image_file_name: &PathBuf,
    recompress: bool,
    action: impl FnOnce(&PathBuf) -> Result<()>,
) -> Result<()> {
    anyhow::ensure!(
        image_file_name.exists(),
        "image doesn't exist: {}",
        image_file_name.to_str().unwrap()
    );

    debug!("Detecting magic for {}", image_file_name.to_string_lossy());
    let detector = Magic::open(Default::default()).context("libmagic open failed")?;

    detector
        .load::<String>(&[])
        .context("libmagic load failed")?;

    let magic = detector.file(image_file_name)?;

    for elem in COMPRESSION_TABLE {
        if magic.contains(elem.marker) {
            info!("Compressed image file found, decompressing...");
            let new_image_file = decompress(image_file_name, elem.extension, elem.generator)?;
            debug!("Decompressed to {}", new_image_file.to_string_lossy());
            action(&new_image_file)?;
            if recompress {
                info!(
                    "Recompressing image from {} to {}",
                    new_image_file.to_string_lossy(),
                    image_file_name.to_string_lossy()
                );
                let success = compress(&new_image_file, image_file_name, elem.generator);

                remove_file(new_image_file).context("Deleting temporary file failed")?;
                success.context("Recompressing failed")?;
                debug!("Compression complete.");
            }

            return Ok(());
        }
    }
    action(image_file_name)
}

fn decompress(
    image_file_name: &PathBuf,
    extension: &'static str,
    generator: &'static dyn CompressionGenerator,
) -> Result<PathBuf, std::io::Error> {
    let mut new_image_file = image_file_name.to_path_buf();
    new_image_file.set_extension(extension);
    let mut destination = File::create(new_image_file.clone())?;
    let mut source = File::open(image_file_name)?;
    let bytes_written = generator.decompress(&mut source, &mut destination)?;
    debug!("Decompress: copied {} bytes.", bytes_written);
    Ok(new_image_file)
}

fn compress(
    uncompressed_file_name: &PathBuf,
    compressed_file_name: &Path,
    generator: &'static dyn CompressionGenerator,
) -> Result<(), std::io::Error> {
    let mut destination = File::create(compressed_file_name)?;
    let mut source = File::open(uncompressed_file_name)?;
    let bytes_written = generator.compress(&mut source, &mut destination)?;
    debug!("Compress: copied {} bytes.", bytes_written);
    Ok(())
}
