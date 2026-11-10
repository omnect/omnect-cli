use crate::file::{COPY_CHUNK_SIZE, IMAGE_BLOCK_SIZE, sparse};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    ops::{Range, RangeInclusive},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
};

const BMAP_EXTENSION: &str = "bmap";
const SHA256_CHECKSUM_TYPE: &str = "sha256";
const SHA256_HEX_LEN: usize = 64;
const BMAP_VERSION: &str = "2.0";
const BMAP_MAJOR_VERSION: u32 = 2;
const BMAP_ROOT: &str = "bmap";
// far above the bmap of a large image, so an image passed as bmap is not read into memory
const MAX_BMAP_SIZE: u64 = 64 * 1024 * 1024;

/// The block ranges of an image that a bmap file maps.
#[derive(Debug)]
pub struct Bmap {
    pub block_size: u64,
    pub image_size: u64,
    pub ranges: Vec<RangeInclusive<u64>>,
    /// The sha256 of each range, if the bmap has them.
    pub sha256: Option<Vec<String>>,
}

impl Bmap {
    pub fn read(path: &Path) -> Result<Self> {
        let file =
            fs::File::open(path).context(format!("Bmap::read: cannot open {}", path.display()))?;
        let too_large = || {
            anyhow::anyhow!(
                "Bmap::read: {} is larger than {MAX_BMAP_SIZE} bytes, so it is not a bmap",
                path.display()
            )
        };
        let len = file
            .metadata()
            .context(format!("Bmap::read: cannot stat {}", path.display()))?
            .len();
        if len > MAX_BMAP_SIZE {
            return Err(too_large());
        }
        let mut bytes = vec![];
        // the size from stat is 0 for e.g. a device or a pipe
        file.take(MAX_BMAP_SIZE + 1)
            .read_to_end(&mut bytes)
            .context(format!("Bmap::read: cannot read {}", path.display()))?;
        if bytes.len() as u64 > MAX_BMAP_SIZE {
            return Err(too_large());
        }
        let text = String::from_utf8(bytes).context(format!(
            "Bmap::read: {} is not UTF-8, so it is not a bmap",
            path.display()
        ))?;
        let doc = roxmltree::Document::parse(&text)
            .context(format!("Bmap::read: invalid xml in {}", path.display()))?;
        let root = doc.root_element();
        anyhow::ensure!(
            root.has_tag_name(BMAP_ROOT),
            "Bmap::read: {} is not a bmap",
            path.display()
        );
        let major_version = root
            .attribute("version")
            .and_then(|v| v.split('.').next()?.parse::<u32>().ok())
            .context(format!(
                "Bmap::read: no valid version in {}",
                path.display()
            ))?;
        // like bmaptool, refuse a bmap of a newer major version
        anyhow::ensure!(
            major_version <= BMAP_MAJOR_VERSION,
            "Bmap::read: {} has version {major_version}.x, only up to {BMAP_MAJOR_VERSION}.x is \
             supported",
            path.display()
        );
        let child = |name: &str| {
            root.children()
                .find(|n| n.has_tag_name(name))
                .context(format!("Bmap::read: no {name} in {}", path.display()))
        };
        let number = |name: &str| -> Result<u64> {
            child(name)?
                .text()
                .unwrap_or_default()
                .trim()
                .parse()
                .context(format!("Bmap::read: invalid {name} in {}", path.display()))
        };

        let block_size = number("BlockSize")?;
        anyhow::ensure!(block_size > 0, "Bmap::read: BlockSize is 0");
        let image_size = number("ImageSize")?;
        let range_nodes: Vec<_> = child("BlockMap")?
            .children()
            .filter(|n| n.has_tag_name("Range"))
            .collect();
        let ranges = range_nodes
            .iter()
            .map(|n| parse_range(n.text().unwrap_or_default()))
            .collect::<Result<_>>()?;
        let sha256 = child("ChecksumType")
            .ok()
            .and_then(|n| n.text())
            .is_some_and(|t| t.trim() == SHA256_CHECKSUM_TYPE)
            .then(|| {
                range_nodes
                    .iter()
                    .map(|n| n.attribute("chksum").map(str::to_owned))
                    .collect::<Option<_>>()
            })
            .flatten();

        Ok(Self {
            block_size,
            image_size,
            ranges,
            sha256,
        })
    }
}

fn parse_range(text: &str) -> Result<RangeInclusive<u64>> {
    let text = text.trim();
    let (first, last) = text.split_once('-').unwrap_or((text, text));
    let parse = |s: &str| {
        s.trim()
            .parse::<u64>()
            .context(format!("parse_range: invalid range {text}"))
    };
    let (first, last) = (parse(first)?, parse(last)?);
    anyhow::ensure!(first <= last, "parse_range: invalid range {text}");

    Ok(first..=last)
}

/// Returns `explicit`, else the bmap `bmaptool` uses for `image`: `<image>.bmap`, else the same
/// with one extension of `image` removed at a time, whichever exists first.
pub fn find_input_bmap(image: &Path, explicit: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        anyhow::ensure!(
            path.is_file(),
            "find_input_bmap: {} not found",
            path.display()
        );
        return Ok(Some(path.to_path_buf()));
    }

    Ok(bmaptool_lookup(image)
        .into_iter()
        .find(|bmap| bmap.is_file()))
}

/// The paths where `bmaptool` looks for the bmap of `image`, in its order: `<image>.bmap`, then
/// the same with one extension of `image` removed at a time.
pub fn bmaptool_lookup(image: &Path) -> Vec<PathBuf> {
    let mut image = image.to_path_buf();
    let mut bmaps = vec![bmap_path(&image)];
    while image.extension().is_some() {
        image.set_extension("");
        bmaps.push(bmap_path(&image));
    }
    bmaps
}

pub fn bmap_path(image: &Path) -> PathBuf {
    image.with_added_extension(BMAP_EXTENSION)
}

/// Fails if a range of `image_file` does not match its checksum in `bmap`, so the bmap of
/// another image with the same size is not used.
pub fn verify_checksums(image_file: &Path, bmap: &Bmap) -> Result<()> {
    let sha256 = bmap
        .sha256
        .as_ref()
        .context("verify_checksums: the bmap has no sha256 range checksums")?;
    let image = fs::File::open(image_file).context(format!(
        "verify_checksums: cannot open {}",
        image_file.display()
    ))?;
    let image_len = image
        .metadata()
        .context("verify_checksums: cannot get image size")?
        .len();
    check_layout(bmap, image_len)?;

    let mut buf = vec![0u8; COPY_CHUNK_SIZE];
    for (range, expected) in bmap.ranges.iter().zip(sha256) {
        let bytes =
            range.start() * bmap.block_size..((range.end() + 1) * bmap.block_size).min(image_len);
        let actual = range_sha256(&image, bytes, &mut buf)?;
        anyhow::ensure!(
            actual.eq_ignore_ascii_case(expected.trim()),
            "verify_checksums: range {range:?} of the image does not match the bmap, so it is the \
             bmap of another image or of an older version of this image"
        );
    }

    Ok(())
}

/// Writes a bmap of `image_file` to `out`. It maps the ranges of `input`, or the whole
/// image without one, and every block that the image file stores as data. A range of `input`
/// can be all zeros and a hole in the image, and a bmap flash still has to write it, else the
/// device keeps what an earlier installation left there.
pub fn write_bmap(image_file: &Path, input: Option<&Bmap>, out: &mut impl Write) -> Result<()> {
    let image = fs::File::open(image_file)
        .context(format!("write_bmap: cannot open {}", image_file.display()))?;
    let image_len = image
        .metadata()
        .context("write_bmap: cannot get image size")?
        .len();
    let block_size = IMAGE_BLOCK_SIZE as u64;

    let ranges = mapped_ranges(&image, image_len, input)?;

    let mut buf = vec![0u8; COPY_CHUNK_SIZE];
    let mut block_map = String::new();
    for range in &ranges {
        let bytes = range.start() * block_size..((range.end() + 1) * block_size).min(image_len);
        let sha256 = range_sha256(&image, bytes, &mut buf)?;
        block_map.push_str(&format!(
            "        <Range chksum=\"{sha256}\"> {}-{} </Range>\n",
            range.start(),
            range.end()
        ));
    }
    let blocks = image_len.div_ceil(block_size);
    let mapped: u64 = ranges.iter().map(|r| r.end() - r.start() + 1).sum();

    // like bmaptool, the file checksum is taken with zeros in place of itself
    let zeros = "0".repeat(SHA256_HEX_LEN);
    let xml = format!(
        "<?xml version=\"1.0\" ?>\n\
         <bmap version=\"{BMAP_VERSION}\">\n    \
         <ImageSize> {image_len} </ImageSize>\n    \
         <BlockSize> {block_size} </BlockSize>\n    \
         <BlocksCount> {blocks} </BlocksCount>\n    \
         <MappedBlocksCount> {mapped} </MappedBlocksCount>\n    \
         <ChecksumType> {SHA256_CHECKSUM_TYPE} </ChecksumType>\n    \
         <BmapFileChecksum> {zeros} </BmapFileChecksum>\n    \
         <BlockMap>\n{block_map}    </BlockMap>\n\
         </bmap>\n"
    );
    let xml = xml.replacen(&zeros, &hex(&Sha256::digest(xml.as_bytes())), 1);

    out.write_all(xml.as_bytes())
        .context("write_bmap: cannot write the bmap")
}

fn mapped_ranges(
    image: &fs::File,
    image_len: u64,
    input: Option<&Bmap>,
) -> Result<Vec<RangeInclusive<u64>>> {
    let block_size = IMAGE_BLOCK_SIZE as u64;
    let mut ranges: Vec<Range<u64>> = match input {
        Some(bmap) => {
            check_layout(bmap, image_len)?;
            bmap.ranges
                .iter()
                .map(|r| r.start() * bmap.block_size..(r.end() + 1) * bmap.block_size)
                .collect()
        }
        None => vec![Range {
            start: 0,
            end: image_len,
        }],
    };
    ranges.extend(sparse::data_ranges(image, 0, image_len)?);
    Ok(sparse::merge(
        ranges
            .into_iter()
            .map(|r| {
                r.start / block_size * block_size..r.end.min(image_len).next_multiple_of(block_size)
            })
            .filter(|r| !r.is_empty())
            .collect(),
    )
    .into_iter()
    .map(|r| r.start / block_size..=r.end / block_size - 1)
    .collect())
}

fn range_sha256(image: &fs::File, bytes: Range<u64>, buf: &mut [u8]) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut pos = bytes.start;
    while pos < bytes.end {
        let n = usize::try_from((bytes.end - pos).min(buf.len() as u64))?;
        image
            .read_exact_at(&mut buf[..n], pos)
            .context("range_sha256: cannot read image")?;
        hasher.update(&buf[..n]);
        pos += n as u64;
    }

    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Fails if `bmap` is for an image of another size or maps blocks outside the image.
fn check_layout(bmap: &Bmap, image_len: u64) -> Result<()> {
    anyhow::ensure!(
        bmap.image_size == image_len,
        "check_layout: bmap is for an image of {} bytes, the image has {image_len}",
        bmap.image_size
    );
    let image_blocks = image_len.div_ceil(bmap.block_size);
    if let Some(range) = bmap.ranges.iter().find(|r| *r.end() >= image_blocks) {
        anyhow::bail!("check_layout: range {range:?} is outside the image");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::file::bmap::*;

    const BLOCK_SIZE: u64 = IMAGE_BLOCK_SIZE as u64;
    // reads zeros without end and reports the size 0
    const ZERO_DEVICE: &str = "/dev/zero";
    const IMAGE_BLOCKS: u64 = 16;
    const DATA_BLOCK: u64 = 0;
    const ZERO_BLOCKS: RangeInclusive<u64> = 2..=3;
    const UNMAPPED_BLOCK: u64 = 5;
    const SMALL_BLOCK_SIZE: u64 = 512;
    const PARTIAL_BLOCK_LEN: usize = 100;
    const DATA_BYTE: u8 = 0x5a;
    // content an earlier installation left on the device
    const OLD_CONTENT_BYTE: u8 = 0xa5;
    const YOCTO_BMAP: &str = "testfiles/image.wic.bmap";
    const YOCTO_IMAGE_SIZE: u64 = 30408704;
    const YOCTO_MAPPED_BLOCKS: u64 = 77;

    fn bmap_xml(image_size: u64, block_map: &str) -> String {
        format!(
            "<?xml version=\"1.0\" ?><bmap version=\"2.0\"><ImageSize>{image_size}</ImageSize>\
             <BlockSize>{BLOCK_SIZE}</BlockSize><BlockMap>{block_map}</BlockMap></bmap>"
        )
    }

    fn write_input_bmap(path: &Path, image_size: u64, ranges: &[RangeInclusive<u64>]) {
        let block_map: String = ranges
            .iter()
            .map(|r| format!("<Range>{}-{}</Range>", r.start(), r.end()))
            .collect();
        fs::write(path, bmap_xml(image_size, &block_map)).expect("write bmap");
    }

    /// A sparse image with data in `DATA_BLOCK` and holes everywhere else.
    fn sparse_image(dir: &Path) -> PathBuf {
        let path = dir.join("image.wic");
        let image = fs::File::create(&path).expect("create image");
        image
            .set_len(IMAGE_BLOCKS * BLOCK_SIZE)
            .expect("set image size");
        image
            .write_all_at(&[DATA_BYTE; IMAGE_BLOCK_SIZE], DATA_BLOCK * BLOCK_SIZE)
            .expect("write data block");
        path
    }

    fn created_bmap(image: &Path, input: Option<&Bmap>) -> Bmap {
        let path = image.with_extension(BMAP_EXTENSION);
        let mut file = fs::File::create(&path).expect("create bmap");
        write_bmap(image, input, &mut file).expect("write bmap");
        Bmap::read(&path).expect("read bmap")
    }

    fn is_mapped(bmap: &Bmap, block: u64) -> bool {
        bmap.ranges.iter().any(|r| r.contains(&block))
    }

    /// Writes the blocks `bmap` maps onto a device that still holds old content, as a bmap
    /// flash does.
    fn bmap_flash(image: &Path, bmap: &Bmap, device: &Path) {
        let image = fs::read(image).expect("read image");
        let mut content = vec![OLD_CONTENT_BYTE; image.len()];
        for range in &bmap.ranges {
            let start = usize::try_from(range.start() * bmap.block_size).expect("start");
            let end = usize::try_from((range.end() + 1) * bmap.block_size).expect("end");
            let end = end.min(image.len());
            content[start..end].copy_from_slice(&image[start..end]);
        }
        fs::write(device, content).expect("write device");
    }

    #[test]
    fn read_parses_yocto_bmap() {
        let bmap = Bmap::read(Path::new(YOCTO_BMAP)).expect("read bmap");

        assert_eq!(bmap.block_size, BLOCK_SIZE);
        assert_eq!(bmap.image_size, YOCTO_IMAGE_SIZE);
        let mapped: u64 = bmap.ranges.iter().map(|r| r.end() - r.start() + 1).sum();
        assert_eq!(mapped, YOCTO_MAPPED_BLOCKS);
        assert_eq!(
            bmap.sha256.map(|c| c.len()),
            Some(bmap.ranges.len()),
            "one checksum per range"
        );
    }

    #[test]
    fn read_rejects_a_file_larger_than_a_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("image.wic.bmap");
        fs::File::create(&path)
            .and_then(|file| file.set_len(MAX_BMAP_SIZE + 1))
            .expect("create sparse file");

        for path in [path.as_path(), Path::new(ZERO_DEVICE)] {
            let error = Bmap::read(path).expect_err("too large");
            assert!(
                error.to_string().contains("is larger than"),
                "{}: {error}",
                path.display()
            );
        }
    }

    #[test]
    fn read_rejects_a_file_that_is_not_utf8() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("image.wic.bmap");
        // as e.g. a compressed image
        fs::write(&path, [u8::MAX]).expect("write file");

        let error = Bmap::read(&path).expect_err("not UTF-8");
        assert!(error.to_string().contains("is not UTF-8"), "{error}");
    }

    #[test]
    fn read_parses_single_blocks_and_spans() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("image.wic.bmap");
        let block_map = "<Range chksum=\"0\"> 3 </Range><Range chksum=\"0\"> 5-7 </Range>";
        fs::write(&path, bmap_xml(BLOCK_SIZE, block_map)).expect("write bmap");

        let bmap = Bmap::read(&path).expect("read bmap");
        assert_eq!(bmap.ranges, vec![3..=3, 5..=7]);
    }

    #[test]
    fn read_has_no_checksums_without_sha256_checksum_type() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("image.wic.bmap");
        fs::write(&path, bmap_xml(BLOCK_SIZE, "<Range>0</Range>")).expect("write bmap");

        assert_eq!(Bmap::read(&path).expect("read bmap").sha256, None);
    }

    #[test]
    fn read_rejects_invalid_bmaps() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("image.wic.bmap");
        let invalid = [
            "<bmap><ImageSize>1</ImageSize><BlockMap/></bmap>".to_string(),
            bmap_xml(BLOCK_SIZE, "<Range>7-5</Range>"),
            bmap_xml(BLOCK_SIZE, "<Range>x</Range>"),
            "<bmap>".to_string(),
            bmap_xml(BLOCK_SIZE, "")
                .replace("<bmap ", "<map ")
                .replace("</bmap>", "</map>"),
            bmap_xml(BLOCK_SIZE, "").replace("version=\"2.0\"", "version=\"3.0\""),
            bmap_xml(BLOCK_SIZE, "").replace(" version=\"2.0\"", ""),
        ];

        for xml in invalid {
            fs::write(&path, &xml).expect("write bmap");
            assert!(Bmap::read(&path).is_err(), "accepted {xml}");
        }
    }

    #[test]
    fn find_input_bmap_takes_explicit_path() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        let explicit = dir.path().join("other.bmap");

        assert!(find_input_bmap(&image, Some(&explicit)).is_err());
        fs::write(&explicit, "").expect("write bmap");
        assert_eq!(
            find_input_bmap(&image, Some(&explicit)).expect("find bmap"),
            Some(explicit)
        );
    }

    #[test]
    fn find_input_bmap_looks_next_to_the_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic.xz");
        let uncompressed_bmap = dir.path().join("image.wic.bmap");
        let compressed_bmap = dir.path().join("image.wic.xz.bmap");

        assert_eq!(find_input_bmap(&image, None).expect("find bmap"), None);
        fs::write(&uncompressed_bmap, "").expect("write bmap");
        assert_eq!(
            find_input_bmap(&image, None).expect("find bmap"),
            Some(uncompressed_bmap)
        );
        fs::write(&compressed_bmap, "").expect("write bmap");
        assert_eq!(
            find_input_bmap(&image, None).expect("find bmap"),
            Some(compressed_bmap)
        );
    }

    #[test]
    fn find_input_bmap_strips_extensions_like_bmaptool() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let bmap = dir.path().join("image.bmap");
        fs::write(&bmap, "").expect("write bmap");

        for image in ["image.wic", "image.wic.xz"] {
            assert_eq!(
                find_input_bmap(&dir.path().join(image), None).expect("find bmap"),
                Some(bmap.clone())
            );
        }
    }

    #[test]
    fn write_bmap_maps_zero_ranges_of_the_input_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());
        let input_bmap = dir.path().join("input.bmap");
        write_input_bmap(&input_bmap, IMAGE_BLOCKS * BLOCK_SIZE, &[ZERO_BLOCKS]);
        let input = Bmap::read(&input_bmap).expect("read bmap");

        let bmap = created_bmap(&image, Some(&input));
        assert!(ZERO_BLOCKS.clone().all(|b| is_mapped(&bmap, b)));
        assert!(
            is_mapped(&bmap, DATA_BLOCK),
            "data outside the input ranges"
        );
        assert!(!is_mapped(&bmap, UNMAPPED_BLOCK));
        let file = fs::File::open(&image).expect("open image");
        assert_eq!(
            sparse::data_ranges(&file, 0, IMAGE_BLOCKS * BLOCK_SIZE).expect("data ranges"),
            vec![DATA_BLOCK * BLOCK_SIZE..(DATA_BLOCK + 1) * BLOCK_SIZE],
            "image is not sparse any more"
        );

        let device = dir.path().join("device");
        bmap_flash(&image, &bmap, &device);
        let device = fs::read(&device).expect("read device");
        let zero_start = usize::try_from(ZERO_BLOCKS.start() * BLOCK_SIZE).expect("start");
        let zero_end = usize::try_from((ZERO_BLOCKS.end() + 1) * BLOCK_SIZE).expect("end");
        assert!(device[zero_start..zero_end].iter().all(|b| *b == 0));
    }

    #[test]
    fn write_bmap_without_input_maps_whole_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());

        assert_eq!(
            created_bmap(&image, None).ranges,
            vec![0..=IMAGE_BLOCKS - 1]
        );
    }

    #[test]
    fn write_bmap_maps_whole_blocks_for_a_smaller_bmap_block_size() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());
        // starts in DATA_BLOCK and ends in the hole after it
        let first = (DATA_BLOCK + 1) * BLOCK_SIZE / SMALL_BLOCK_SIZE - 1;
        let input = Bmap {
            block_size: SMALL_BLOCK_SIZE,
            image_size: IMAGE_BLOCKS * BLOCK_SIZE,
            ranges: vec![RangeInclusive::new(first, first + 2)],
            sha256: None,
        };

        assert!(is_mapped(
            &created_bmap(&image, Some(&input)),
            DATA_BLOCK + 1
        ));
    }

    #[test]
    fn verify_checksums_accepts_bmap_of_the_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());
        // a mapped last block that is shorter than the block size
        fs::OpenOptions::new()
            .write(true)
            .open(&image)
            .expect("open image")
            .write_all_at(&[DATA_BYTE; PARTIAL_BLOCK_LEN], IMAGE_BLOCKS * BLOCK_SIZE)
            .expect("write partial block");

        verify_checksums(&image, &created_bmap(&image, None)).expect("verify checksums");
    }

    #[test]
    fn verify_checksums_rejects_changed_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());
        let bmap = created_bmap(&image, None);
        fs::OpenOptions::new()
            .write(true)
            .open(&image)
            .expect("open image")
            .write_all_at(&[!DATA_BYTE], DATA_BLOCK * BLOCK_SIZE)
            .expect("change data block");

        let err = verify_checksums(&image, &bmap).expect_err("changed image accepted");
        assert!(err.to_string().contains("does not match the bmap"), "{err}");
    }

    #[test]
    fn verify_checksums_rejects_bmap_without_checksums() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());
        let mut bmap = created_bmap(&image, None);
        bmap.sha256 = None;

        assert!(verify_checksums(&image, &bmap).is_err());
    }

    #[test]
    fn bmap_of_another_image_is_rejected() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = sparse_image(dir.path());
        let mut other_size = created_bmap(&image, None);
        other_size.image_size += BLOCK_SIZE;
        let outside = Bmap {
            block_size: BLOCK_SIZE,
            image_size: IMAGE_BLOCKS * BLOCK_SIZE,
            ranges: vec![RangeInclusive::new(IMAGE_BLOCKS, IMAGE_BLOCKS)],
            sha256: Some(vec![String::new()]),
        };

        for bmap in [&other_size, &outside] {
            assert!(verify_checksums(&image, bmap).is_err());
            assert!(write_bmap(&image, Some(bmap), &mut Vec::new()).is_err());
        }
    }
}
