use crate::file::{COPY_CHUNK_SIZE, IMAGE_BLOCK_SIZE, is_zero};
use anyhow::{Context, Result};
use filemagic::Magic;
use log::debug;
use std::{
    env,
    fs::File,
    io::{BufWriter, IntoInnerError, Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    str::FromStr,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

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
        // only `finish` reports an error while writing the trailer
        let (bytes_read, _) = match &self {
            Compression::bzip2 => {
                let mut enc = bzip2::write::BzEncoder::new(destination, bzip2::Compression::best());
                (std::io::copy(source, &mut enc)?, enc.finish()?)
            }
            Compression::gzip => {
                let mut enc =
                    flate2::write::GzEncoder::new(destination, flate2::Compression::best());
                (std::io::copy(source, &mut enc)?, enc.finish()?)
            }
            Compression::xz {
                compression_level: level,
            } => {
                let stream = xz2::stream::MtStreamBuilder::new()
                    .threads(num_cpus::get() as u32)
                    .preset(*level)
                    .encoder()?;
                let mut enc = xz2::write::XzEncoder::new_stream(destination, stream);
                (std::io::copy(source, &mut enc)?, enc.finish()?)
            }
        };
        Ok(bytes_read)
    }

    pub fn decompress(
        &self,
        source: &mut std::fs::File,
        destination: &mut std::fs::File,
    ) -> std::io::Result<u64> {
        // the multi decoders also read concatenated streams, as pbzip2 writes them
        let mut decoder: Box<dyn Read + '_> = match &self {
            Compression::bzip2 => Box::new(bzip2::read::MultiBzDecoder::new(source)),
            Compression::gzip => Box::new(flate2::read::MultiGzDecoder::new(source)),
            Compression::xz { .. } => Box::new(xz2::read::XzDecoder::new_multi_decoder(source)),
        };
        let mut sparse = BufWriter::with_capacity(COPY_CHUNK_SIZE, SparseWriter::new(destination)?);
        let bytes_written = std::io::copy(&mut decoder, &mut sparse)?;
        sparse
            .into_inner()
            .map_err(IntoInnerError::into_error)?
            .finish()?;
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
        self.extensions()[0]
    }

    /// The first one is used for a compressed image that omnect-cli creates.
    fn extensions(&self) -> &'static [&'static str] {
        match &self {
            Compression::bzip2 => &["bz2", "bzip2"],
            Compression::gzip => &["gz", "gzip"],
            Compression::xz { .. } => &["xz"],
        }
    }

    /// `path` with the extension omnect-cli gives a compressed image appended.
    pub fn compressed_path(&self, path: &Path) -> PathBuf {
        path.with_added_extension(self.extension())
    }

    pub fn matches_extension(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| self.extensions().contains(&e))
    }

    /// Whether `path` has the extension of `compressed_path`, which `bmaptool` reads.
    pub fn matches_output_extension(&self, path: &Path) -> bool {
        path.extension().is_some_and(|e| e == self.extension())
    }

    pub fn has_extension(path: &Path) -> bool {
        Compression::iter().any(|c| c.matches_extension(path))
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

/// Skips zero blocks, so they stay holes in the file.
struct SparseWriter<'a> {
    file: &'a mut File,
    pos: u64,
}

impl<'a> SparseWriter<'a> {
    fn new(file: &'a mut File) -> std::io::Result<Self> {
        let pos = file.stream_position()?;
        // skipped zero blocks must read back as zeros
        file.set_len(pos)?;
        Ok(Self { file, pos })
    }

    fn finish(self) -> std::io::Result<()> {
        // extends the file over zero blocks at the end
        self.file.set_len(self.pos)?;
        self.file.seek(SeekFrom::Start(self.pos))?;
        Ok(())
    }
}

impl Write for SparseWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let block_offset = (self.pos % IMAGE_BLOCK_SIZE as u64) as usize;

        // one call handles a run of blocks that are all zero or all data
        let mut end = buf.len().min(IMAGE_BLOCK_SIZE - block_offset);
        let zero = is_zero(&buf[..end]);
        while end < buf.len() {
            let next = buf.len().min(end + IMAGE_BLOCK_SIZE);
            if is_zero(&buf[end..next]) != zero {
                break;
            }
            end = next;
        }

        let written = if zero {
            end
        } else {
            self.file.write_at(&buf[..end], self.pos)?
        };
        self.pos += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// Decompresses `image_file_name` into `dest_dir`, named without the compression extension.
pub fn decompress(
    image_file_name: &Path,
    compression: &Compression,
    dest_dir: &Path,
) -> Result<PathBuf> {
    anyhow::ensure!(
        compression.matches_extension(image_file_name),
        "decompress: {} has no {:?} file extension",
        image_file_name.display(),
        compression.extensions()
    );
    let new_image_file = dest_dir.join(
        image_file_name
            .file_stem()
            .context("decompress: cannot get image file name")?,
    );

    let mut source = File::open(image_file_name).context(format!(
        "decompress: cannot open {}",
        image_file_name.display()
    ))?;
    let mut destination = File::create(&new_image_file).context(format!(
        "decompress: cannot create {}",
        new_image_file.display()
    ))?;
    debug!("decompress {image_file_name:?} to {new_image_file:?}");
    let bytes_written = compression.decompress(&mut source, &mut destination)?;
    debug!("image::decompress: copied {} bytes.", bytes_written);
    Ok(new_image_file)
}

pub fn compress(
    image_file_name: &Path,
    compression: &Compression,
    destination: &mut File,
) -> Result<()> {
    let mut source = File::open(image_file_name).context(format!(
        "compress: cannot open {}",
        image_file_name.display()
    ))?;
    debug!("compress {image_file_name:?}");
    let bytes_written = compression.compress(&mut source, destination)?;
    debug!("image::compress: copied {} bytes.", bytes_written);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::file::compression::*;
    use std::{fs, io::Read, os::unix::fs::MetadataExt};

    // unit of `st_blocks`
    const STAT_BLOCK_SIZE: u64 = 512;
    const DATA_OFFSET: usize = 1024 * 1024 + 100;
    const DATA: &[u8] = b"abc";
    const IMAGE_LEN: usize = 3 * 1024 * 1024 + 123;
    const XZ_TEST_LEVEL: u32 = 1;
    // cuts into the trailer, which every format checks only at the end of the stream
    const TRUNCATED_BYTES: u64 = 4;
    const OLD_CONTENT_BYTE: u8 = 0xa5;
    const OLD_CONTENT_EXTRA_LEN: usize = 4096;

    fn all_compressions() -> [Compression; 3] {
        [
            Compression::bzip2,
            Compression::gzip,
            Compression::xz {
                compression_level: XZ_TEST_LEVEL,
            },
        ]
    }

    fn test_image() -> Vec<u8> {
        let mut image = vec![0u8; IMAGE_LEN];
        image[DATA_OFFSET..DATA_OFFSET + DATA.len()].copy_from_slice(DATA);
        image
    }

    fn compress_image(compression: &Compression, image: &[u8]) -> File {
        let mut source = tempfile::tempfile().expect("create source");
        source.write_all(image).expect("write source");
        source.rewind().expect("rewind source");

        let mut compressed = tempfile::tempfile().expect("create compressed");
        compression
            .compress(&mut source, &mut compressed)
            .expect("compress");
        compressed.rewind().expect("rewind compressed");
        compressed
    }

    fn read_all(file: &mut File) -> Vec<u8> {
        let mut content = Vec::new();
        file.rewind().expect("rewind");
        file.read_to_end(&mut content).expect("read");
        content
    }

    #[test]
    fn decompress_keeps_zero_blocks_as_holes() {
        let image = test_image();

        for compression in all_compressions() {
            let mut compressed = compress_image(&compression, &image);
            let mut decompressed = tempfile::tempfile().expect("create decompressed");
            compression
                .decompress(&mut compressed, &mut decompressed)
                .expect("decompress");

            assert!(
                read_all(&mut decompressed) == image,
                "{compression:?}: content differs"
            );
            let meta = decompressed.metadata().expect("metadata");
            assert!(
                meta.blocks() * STAT_BLOCK_SIZE < meta.len(),
                "{compression:?}: no holes"
            );
        }
    }

    #[test]
    fn decompress_replaces_old_content() {
        let image = test_image();

        for compression in all_compressions() {
            let mut compressed = compress_image(&compression, &image);
            let mut decompressed = tempfile::tempfile().expect("create decompressed");
            decompressed
                .write_all(&vec![OLD_CONTENT_BYTE; IMAGE_LEN + OLD_CONTENT_EXTRA_LEN])
                .expect("write old content");
            decompressed.rewind().expect("rewind decompressed");

            compression
                .decompress(&mut compressed, &mut decompressed)
                .expect("decompress");

            assert!(
                read_all(&mut decompressed) == image,
                "{compression:?}: content differs"
            );
        }
    }

    #[test]
    fn decompress_rejects_truncated_input() {
        let image = test_image();

        for compression in all_compressions() {
            let mut compressed = compress_image(&compression, &image);
            let len = compressed.metadata().expect("metadata").len();
            compressed
                .set_len(len - TRUNCATED_BYTES)
                .expect("truncate compressed");

            let mut decompressed = tempfile::tempfile().expect("create decompressed");
            assert!(
                compression
                    .decompress(&mut compressed, &mut decompressed)
                    .is_err(),
                "{compression:?}: truncated input accepted"
            );
        }
    }

    #[test]
    fn decompress_reads_concatenated_streams() {
        let image = test_image();
        let (first, second) = image.split_at(DATA_OFFSET);

        for compression in all_compressions() {
            let mut concatenated = tempfile::tempfile().expect("create concatenated");
            for part in [first, second] {
                let mut compressed = compress_image(&compression, part);
                concatenated
                    .write_all(&read_all(&mut compressed))
                    .expect("write stream");
            }
            concatenated.rewind().expect("rewind concatenated");

            let mut decompressed = tempfile::tempfile().expect("create decompressed");
            compression
                .decompress(&mut concatenated, &mut decompressed)
                .expect("decompress");
            assert!(
                read_all(&mut decompressed) == image,
                "{compression:?}: content differs"
            );
        }
    }

    #[test]
    fn decompress_file_strips_every_extension_of_the_compression() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = test_image();

        for compression in all_compressions() {
            for extension in compression.extensions() {
                let path = dir.path().join(format!("image.wic.{extension}"));
                let mut compressed = compress_image(&compression, &image);
                fs::write(&path, read_all(&mut compressed)).expect("write compressed");

                let decompressed = decompress(&path, &compression, dir.path()).expect("decompress");
                assert_eq!(decompressed, dir.path().join("image.wic"));
                assert!(
                    fs::read(&decompressed).expect("read decompressed") == image,
                    "{compression:?}: content of .{extension} differs"
                );
                assert!(Compression::has_extension(&path));
            }
        }
    }

    #[test]
    fn compressed_path_has_an_extension_bmaptool_reads() {
        let image = Path::new("image.wic");
        let expected = ["image.wic.bz2", "image.wic.gz", "image.wic.xz"];

        for (compression, expected) in all_compressions().iter().zip(expected) {
            assert_eq!(compression.compressed_path(image), Path::new(expected));
        }
    }

    #[test]
    fn decompress_file_rejects_unknown_extension() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let compression = Compression::gzip;
        let path = dir.path().join("image.wic");
        let mut compressed = compress_image(&compression, &test_image());
        let content = read_all(&mut compressed);
        fs::write(&path, &content).expect("write compressed");

        assert!(decompress(&path, &compression, dir.path()).is_err());
        assert!(
            fs::read(&path).expect("read compressed") == content,
            "source was changed"
        );
    }

    #[test]
    fn compress_fails_when_the_trailer_cannot_be_written() {
        let dir = tempfile::tempdir().expect("create temp dir");
        // an empty source makes `finish` the first write to the destination
        let source = dir.path().join("empty.wic");
        fs::write(&source, []).expect("write source");
        for compression in all_compressions() {
            let mut source = fs::File::open(&source).expect("open source");
            let mut full = fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full");
            assert!(
                compression.compress(&mut source, &mut full).is_err(),
                "{compression:?}: write error was ignored"
            );
        }
    }
}
