use filemagic::Magic;
use log::{debug, info};
use std::fs::remove_file;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use tokio::fs::File;

trait CompressionGenerator {
    fn compressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a>;
    fn decompressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a>;
}

struct XzGenerator;
impl CompressionGenerator for XzGenerator {
    fn compressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a> {
        Box::new(async_compression::tokio::write::XzEncoder::new(destination))
    }
    fn decompressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a> {
        Box::new(async_compression::tokio::write::XzDecoder::new(destination))
    }
}

struct BzGenerator;
impl CompressionGenerator for BzGenerator {
    fn compressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a> {
        Box::new(async_compression::tokio::write::BzEncoder::new(destination))
    }
    fn decompressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a> {
        Box::new(async_compression::tokio::write::BzDecoder::new(destination))
    }
}

struct GzGenerator;
impl CompressionGenerator for GzGenerator {
    fn compressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a> {
        Box::new(async_compression::tokio::write::GzipEncoder::new(
            destination,
        ))
    }
    fn decompressor<'a>(
        &self,
        destination: &'a mut (dyn tokio::io::AsyncWrite + Unpin),
    ) -> Box<dyn tokio::io::AsyncWrite + Unpin + 'a> {
        Box::new(async_compression::tokio::write::GzipDecoder::new(
            destination,
        ))
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

pub fn validate_and_decompress_image(
    image_file_name: &PathBuf,
    action: impl FnOnce(&PathBuf) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Detecting magic for {}", image_file_name.to_string_lossy());
    let detector = Magic::open(Default::default());
    let detector = match detector {
        Err(e) => {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("libmagic open failed with error {}", e.to_string()),
            )));
        }
        Ok(d) => d,
    };
    match detector.load::<String>(&[]) {
        Err(e) => {
            return Err(Box::new(Error::new(
                ErrorKind::Other,
                format!("libmagic load failed with error {}", e.to_string()),
            )));
        }
        _ => {}
    }
    let magic = detector.file(&image_file_name)?;
    for elem in COMPRESSION_TABLE {
        if magic.find(elem.marker) != None {
            info!("Compressed image file found, decompressing...");
            let new_image_file = decompress(image_file_name, elem.extension, elem.generator)?;
            debug!("Decompressed to {}", new_image_file.to_string_lossy());
            let mut success = action(&new_image_file);
            match success {
                Ok(_n) => {
                    info!(
                        "Recompressing image to {}",
                        image_file_name.to_string_lossy()
                    );
                    match compress(image_file_name, &new_image_file, elem.generator) {
                        Ok(_e) => {
                            debug!("Compression complete.");
                        }
                        Err(e) => {
                            success = Err(Box::new(Error::new(
                                ErrorKind::Other,
                                format!("Recompressing failed with error {}", e.to_string()),
                            )));
                        }
                    }
                }
                _ => {}
            }
            match remove_file(new_image_file) {
                Err(e) => {
                    success = Err(Box::new(Error::new(
                        ErrorKind::Other,
                        format!(
                            "Deleting temporary file failed with error {}",
                            e.to_string()
                        ),
                    )));
                }
                _ => {}
            }
            return success;
        }
    }
    action(image_file_name)
}

#[tokio::main]
async fn decompress(
    image_file_name: &PathBuf,
    extension: &'static str,
    generator: &'static dyn CompressionGenerator,
) -> Result<PathBuf, tokio::io::Error> {
    let mut new_image_file = image_file_name.to_path_buf();
    new_image_file.set_extension(extension);
    let mut destination = File::create(new_image_file.clone()).await?;
    let mut source = File::open(image_file_name).await?;
    let mut decompressor = generator.decompressor(&mut destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    Ok(new_image_file)
}

#[tokio::main]
async fn compress(
    uncompressed_file_name: &PathBuf,
    compressed_file_name: &PathBuf,
    generator: &'static dyn CompressionGenerator,
) -> Result<(), tokio::io::Error> {
    let mut destination = File::create(compressed_file_name.clone()).await?;
    let mut source = File::open(uncompressed_file_name).await?;
    let mut decompressor = generator.compressor(&mut destination);
    tokio::io::copy(&mut source, &mut decompressor).await?;
    Ok(())
}
