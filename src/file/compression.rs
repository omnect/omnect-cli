use anyhow::{Context, Result};
use filemagic::Magic;
use log::debug;
use std::env;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

const SPARSE_BLOCK_SIZE: u64 = 4096;

#[derive(Clone, Debug, EnumIter)]
#[allow(non_camel_case_types)]
pub enum Compression {
    xz { compression_level: u32 },
    bzip2,
    gzip,
}

impl FromStr for Compression {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Compression> {
        match input {
            "xz" => {
                let level = env::var("XZ_COMPRESSION_LEVEL")
                    .unwrap_or_else(|_| "9".to_string())
                    .parse()
                    .unwrap_or(9);

                let level = if (0..=9).contains(&level) { level } else { 4 };

                Ok(Compression::xz {
                    compression_level: level,
                })
            }
            "bzip2" => Ok(Compression::bzip2),
            "gzip" => Ok(Compression::gzip),
            _ => anyhow::bail!("unknown compression: use either xz, bzip2 or gzip"),
        }
    }
}

impl Compression {
    pub fn compress(
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
            Compression::xz {
                compression_level: level,
            } => {
                let stream = xz2::stream::MtStreamBuilder::new()
                    .threads(num_cpus::get() as u32)
                    .preset(*level)
                    .encoder()?;
                Box::new(xz2::write::XzEncoder::new_stream(destination, stream))
            }
        };

        let bytes_written = std::io::copy(source, &mut enc)?;
        enc.flush()?;
        Ok(bytes_written)
    }

    pub fn decompress(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        let sparse = SparseWriter(destination);
        let mut dec: Box<dyn std::io::Write + '_> = match &self {
            Compression::bzip2 => Box::new(bzip2::write::BzDecoder::new(sparse)),
            Compression::gzip => Box::new(flate2::write::GzDecoder::new(sparse)),
            Compression::xz { .. } => Box::new(xz2::write::XzDecoder::new(sparse)),
        };

        let bytes_written = std::io::copy(source, &mut dec)?;
        dec.write_all(&[])?;
        dec.flush()?;
        drop(dec);

        // zero blocks at the end were only seeked over
        let len = destination.stream_position()?;
        destination.set_len(len)?;
        Ok(bytes_written)
    }

    fn marker(&self) -> &'static str {
        match &self {
            Compression::bzip2 => "bzip2 compressed data",
            Compression::gzip => "gzip compressed data",
            Compression::xz { .. } => "XZ compressed data",
        }
    }

    fn extension(&self) -> &'static str {
        match &self {
            Compression::bzip2 => "bzip2",
            Compression::gzip => "gzip",
            Compression::xz { .. } => "xz",
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

/// Seeks over zero blocks instead of writing them, so they stay holes in a new file.
struct SparseWriter<'a>(&'a mut File);

impl Write for SparseWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let pos = self.0.stream_position()?;
        let to_block_end = SPARSE_BLOCK_SIZE - pos % SPARSE_BLOCK_SIZE;
        let chunk = &buf[..buf.len().min(to_block_end as usize)];
        if chunk.iter().all(|b| *b == 0) {
            self.0.seek(SeekFrom::Current(chunk.len() as i64))?;
            Ok(chunk.len())
        } else {
            self.0.write(chunk)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

pub fn decompress(image_file_name: &PathBuf, compression: &Compression) -> Result<PathBuf> {
    let mut new_image_file = PathBuf::from(image_file_name);

    if new_image_file
        .extension()
        .is_some_and(|ext| ext == compression.extension())
    {
        new_image_file.set_extension("");
    }

    let mut destination = File::create(&new_image_file)?;
    let mut source = File::open(image_file_name)?;
    debug!("decompress {image_file_name:?} to {new_image_file:?}");
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
    debug!("compress {image_file_name:?} to {new_image_file:?}");
    let bytes_written = compression.compress(&mut source, &mut destination)?;
    debug!("image::compress: copied {} bytes.", bytes_written);
    Ok(new_image_file)
}

#[cfg(test)]
mod tests {
    use crate::file::compression::*;
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;

    // unit of `st_blocks`
    const STAT_BLOCK_SIZE: u64 = 512;
    const DATA_OFFSET: usize = 1024 * 1024 + 100;
    const IMAGE_LEN: usize = 3 * 1024 * 1024 + 123;

    fn roundtrip(compression: Compression, image: &[u8]) -> File {
        let mut source = tempfile::tempfile().expect("create source");
        source.write_all(image).expect("write source");
        source.rewind().expect("rewind source");

        let mut compressed = tempfile::tempfile().expect("create compressed");
        compression
            .compress(&mut source, &mut compressed)
            .expect("compress");
        compressed.rewind().expect("rewind compressed");

        let mut decompressed = tempfile::tempfile().expect("create decompressed");
        compression
            .decompress(&mut compressed, &mut decompressed)
            .expect("decompress");
        decompressed
    }

    #[test]
    fn decompress_keeps_zero_blocks_as_holes() {
        let mut image = vec![0u8; IMAGE_LEN];
        image[DATA_OFFSET..DATA_OFFSET + 3].copy_from_slice(b"abc");

        for compression in [
            Compression::bzip2,
            Compression::gzip,
            Compression::xz {
                compression_level: 1,
            },
        ] {
            let mut decompressed = roundtrip(compression.clone(), &image);

            let mut content = Vec::new();
            decompressed.rewind().expect("rewind decompressed");
            decompressed
                .read_to_end(&mut content)
                .expect("read decompressed");
            assert!(content == image, "{compression:?}: content differs");

            let meta = decompressed.metadata().expect("metadata");
            assert!(
                meta.blocks() * STAT_BLOCK_SIZE < meta.len(),
                "{compression:?}: no holes"
            );
        }
    }
}
