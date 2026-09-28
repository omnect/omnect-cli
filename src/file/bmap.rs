use crate::file::partition::{SECTOR_SIZE, get_partitions};
use anyhow::{Context, Result};
use log::debug;
use std::fs;
use std::os::unix::fs::FileExt;
use std::process::Command;

const EXT4_SUPERBLOCK_OFFSET: u64 = 1024;
const EXT4_SUPERBLOCK_SIZE: usize = 1024;
const EXT4_S_LOG_BLOCK_SIZE: usize = 0x18;
const EXT4_S_MAGIC: usize = 0x38;
const EXT4_S_FEATURE_COMPAT: usize = 0x5c;
const EXT4_S_JOURNAL_INUM: usize = 0xe0;
const EXT4_MAGIC: u16 = 0xef53;
const EXT4_FEATURE_COMPAT_HAS_JOURNAL: u32 = 0x4;
const EXT4_MIN_BLOCK_SIZE: u64 = 1024;
const EXT4_MAX_LOG_BLOCK_SIZE: u32 = 6;

const DEBUGFS: &str = "debugfs";
// debugfs is installed to sbin, which is not always in the PATH of a normal user
const DEBUGFS_SBIN: &str = "/usr/sbin/debugfs";

struct Ext4Journal {
    inode: u32,
    block_size: u64,
}

fn read_ext4_journal(image: &fs::File, offset: u64) -> Result<Option<Ext4Journal>> {
    let mut sb = [0u8; EXT4_SUPERBLOCK_SIZE];
    image
        .read_exact_at(&mut sb, offset + EXT4_SUPERBLOCK_OFFSET)
        .context("read_ext4_journal: cannot read superblock")?;
    let le16 = |o: usize| u16::from_le_bytes([sb[o], sb[o + 1]]);
    let le32 = |o: usize| u32::from_le_bytes([sb[o], sb[o + 1], sb[o + 2], sb[o + 3]]);

    if le16(EXT4_S_MAGIC) != EXT4_MAGIC
        || le32(EXT4_S_FEATURE_COMPAT) & EXT4_FEATURE_COMPAT_HAS_JOURNAL == 0
    {
        return Ok(None);
    }

    // an external journal has no inode in this filesystem
    let inode = le32(EXT4_S_JOURNAL_INUM);
    if inode == 0 {
        return Ok(None);
    }

    let log_block_size = le32(EXT4_S_LOG_BLOCK_SIZE);
    anyhow::ensure!(
        log_block_size <= EXT4_MAX_LOG_BLOCK_SIZE,
        "read_ext4_journal: invalid block size"
    );
    let block_size = EXT4_MIN_BLOCK_SIZE << log_block_size;

    Ok(Some(Ext4Journal { inode, block_size }))
}

fn ext4_journal_blocks(image_file: &str, offset: u64, inode: u32) -> Result<Vec<u64>> {
    let debugfs_cmd = |program| {
        let mut cmd = Command::new(program);
        cmd.arg("-R")
            .arg(format!("blocks <{inode}>"))
            .arg(format!("{image_file}?offset={offset}"));
        cmd
    };
    let mut debugfs = debugfs_cmd(DEBUGFS);
    let output = match debugfs.output() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debugfs = debugfs_cmd(DEBUGFS_SBIN);
            debugfs.output()
        }
        output => output,
    }
    .context(format!("ext4_journal_blocks: cannot run {debugfs:?}"))?;
    anyhow::ensure!(
        output.status.success(),
        "ext4_journal_blocks: {debugfs:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let blocks = String::from_utf8(output.stdout)
        .context("ext4_journal_blocks: invalid debugfs output")?
        .split_whitespace()
        .map(|b| {
            b.parse::<u64>()
                .context(format!("ext4_journal_blocks: invalid block number {b}"))
        })
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(
        !blocks.is_empty(),
        "ext4_journal_blocks: no journal blocks at offset {offset}"
    );

    Ok(blocks)
}

/// Rewrites the journal blocks of every ext4 partition with their own content. Blocks that
/// are holes become allocated, so a bmap maps the whole journal and a bmap-based flash
/// overwrites the journal an earlier installation left on the device.
fn allocate_ext4_journals(image_file: &str) -> Result<()> {
    let image = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(image_file)
        .context(format!("allocate_ext4_journals: cannot open {image_file}"))?;
    let image_len = image
        .metadata()
        .context("allocate_ext4_journals: cannot get image size")?
        .len();

    for partition in get_partitions(image_file)? {
        let offset = partition.start * SECTOR_SIZE;
        if offset + EXT4_SUPERBLOCK_OFFSET + EXT4_SUPERBLOCK_SIZE as u64 > image_len {
            continue;
        }
        let Some(journal) = read_ext4_journal(&image, offset)? else {
            continue;
        };
        let blocks = ext4_journal_blocks(image_file, offset, journal.inode)?;
        debug!(
            "allocate_ext4_journals: partition {}: {} journal blocks",
            partition.num,
            blocks.len()
        );

        let mut buf = vec![0u8; usize::try_from(journal.block_size)?];
        for block in blocks {
            let pos = offset + block * journal.block_size;
            image
                .read_exact_at(&mut buf, pos)
                .context("allocate_ext4_journals: cannot read journal block")?;
            image
                .write_all_at(&buf, pos)
                .context("allocate_ext4_journals: cannot write journal block")?;
        }
    }

    image
        .sync_all()
        .context("allocate_ext4_journals: cannot sync image")
}

pub fn generate_bmap_file(image_file: &str) -> Result<()> {
    allocate_ext4_journals(image_file)?;

    let mut bmaptool = Command::new("bmaptool");
    bmaptool
        .arg("create")
        .arg("-o")
        .arg(format!("{image_file}.bmap"))
        .arg(image_file);
    let status = bmaptool
        .status()
        .context(format!("generate_bmap_file: cannot run {bmaptool:?}"))?;
    anyhow::ensure!(status.success(), "generate_bmap_file: {bmaptool:?} failed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::file::bmap::*;
    use std::path::Path;

    const PART_START_LBA: u32 = 2048;
    const PART_SECTORS: u32 = 16384;
    const FS_BLOCK_SIZE: u64 = 1024;
    const MBR_DISK_SIGNATURE: [u8; 4] = [1, 2, 3, 4];
    const MBR_TYPE_LINUX: u8 = 0x83;
    const BMAP_BLOCK_SIZE: u64 = 4096;
    // content an earlier installation left on the device
    const OLD_CONTENT_BYTE: u8 = 0xa5;
    const JOURNAL_INODE: u32 = 8;
    const INVALID_LOG_BLOCK_SIZE: u32 = 40;

    fn run(cmd: &mut Command) {
        let status = cmd.status().expect("run command");
        assert!(status.success(), "{cmd:?} failed");
    }

    fn create_image_with_mbr(image: &Path, len: u64) {
        let mut f = fs::File::create(image).expect("create image");
        f.set_len(len).expect("set image size");
        let mut mbr = mbrman::MBR::new_from(&mut f, SECTOR_SIZE as u32, MBR_DISK_SIGNATURE)
            .expect("create mbr");
        mbr[1] = mbrman::MBRPartitionEntry {
            boot: mbrman::BOOT_INACTIVE,
            first_chs: mbrman::CHS::empty(),
            sys: MBR_TYPE_LINUX,
            last_chs: mbrman::CHS::empty(),
            starting_lba: PART_START_LBA,
            sectors: PART_SECTORS,
        };
        mbr.write_into(&mut f).expect("write mbr");
    }

    fn create_image_with_ext4_journal(image: &Path) {
        let raw = image.with_extension("raw");
        create_image_with_mbr(&raw, u64::from(PART_START_LBA + PART_SECTORS) * SECTOR_SIZE);

        let offset = u64::from(PART_START_LBA) * SECTOR_SIZE;
        let fs_blocks = u64::from(PART_SECTORS) * SECTOR_SIZE / FS_BLOCK_SIZE;
        run(Command::new("mkfs.ext4")
            .arg("-q")
            .arg("-F")
            .arg("-b")
            .arg(FS_BLOCK_SIZE.to_string())
            .arg("-J")
            .arg("size=1")
            .arg("-E")
            .arg(format!("offset={offset}"))
            .arg(&raw)
            .arg(fs_blocks.to_string()));
        // mke2fs zeroes the journal as unwritten extents, which a bmap maps; a sparse copy
        // turns the zeros into holes, as a sparse decompress does
        run(Command::new("cp")
            .arg("--sparse=always")
            .arg(&raw)
            .arg(image));
    }

    fn bmap_ranges(bmap: &str) -> Vec<(u64, u64)> {
        bmap.lines()
            .filter_map(|l| {
                let range = l.split_once("<Range")?.1.split_once('>')?.1;
                let range = range.split_once("</Range")?.0.trim();
                let (a, b) = range.split_once('-').unwrap_or((range, range));
                Some((
                    a.parse().expect("range start"),
                    b.parse().expect("range end"),
                ))
            })
            .collect()
    }

    fn unmapped_journal_blocks(image: &str, bmap: &str) -> usize {
        let ranges = bmap_ranges(bmap);
        let offset = u64::from(PART_START_LBA) * SECTOR_SIZE;
        let file = fs::File::open(image).expect("open image");
        let journal = read_ext4_journal(&file, offset)
            .expect("read superblock")
            .expect("ext4 with journal");
        ext4_journal_blocks(image, offset, journal.inode)
            .expect("journal blocks")
            .iter()
            .map(|b| (offset + b * journal.block_size) / BMAP_BLOCK_SIZE)
            .filter(|b| !ranges.iter().any(|(s, e)| s <= b && b <= e))
            .count()
    }

    /// Writes the blocks `bmap` maps onto a device that still holds old content, as a bmap
    /// flash does.
    fn bmap_flash(image: &str, bmap: &str, device: &Path) {
        let image = fs::File::open(image).expect("open image");
        let len = image.metadata().expect("image metadata").len();
        fs::write(
            device,
            vec![OLD_CONTENT_BYTE; usize::try_from(len).expect("image size")],
        )
        .expect("write old content");
        let device = fs::OpenOptions::new()
            .write(true)
            .open(device)
            .expect("open device");

        let mut buf = vec![0u8; usize::try_from(BMAP_BLOCK_SIZE).expect("block size")];
        for (first, last) in bmap_ranges(bmap) {
            for block in first..=last {
                image
                    .read_exact_at(&mut buf, block * BMAP_BLOCK_SIZE)
                    .expect("read image block");
                device
                    .write_all_at(&buf, block * BMAP_BLOCK_SIZE)
                    .expect("write device block");
            }
        }
    }

    fn journal_on_device_matches_image(image: &str, device: &Path) -> bool {
        let offset = u64::from(PART_START_LBA) * SECTOR_SIZE;
        let image_file = fs::File::open(image).expect("open image");
        let device = fs::File::open(device).expect("open device");
        let journal = read_ext4_journal(&image_file, offset)
            .expect("read superblock")
            .expect("ext4 with journal");

        let block_size = usize::try_from(journal.block_size).expect("block size");
        let (mut a, mut b) = (vec![0u8; block_size], vec![0u8; block_size]);
        ext4_journal_blocks(image, offset, journal.inode)
            .expect("journal blocks")
            .iter()
            .all(|block| {
                let pos = offset + block * journal.block_size;
                image_file.read_exact_at(&mut a, pos).expect("read image");
                device.read_exact_at(&mut b, pos).expect("read device");
                a == b
            })
    }

    #[test]
    fn generate_bmap_file_maps_ext4_journal() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_path = dir.path().join("image.wic");
        create_image_with_ext4_journal(&image_path);
        let image = image_path.to_str().expect("image path");
        let bmap_path = format!("{image}.bmap");

        run(Command::new("bmaptool")
            .arg("create")
            .arg("-o")
            .arg(&bmap_path)
            .arg(image));
        let bmap = fs::read_to_string(&bmap_path).expect("read bmap");
        assert!(unmapped_journal_blocks(image, &bmap) > 0);
        let device = dir.path().join("device");
        bmap_flash(image, &bmap, &device);
        assert!(!journal_on_device_matches_image(image, &device));

        generate_bmap_file(image).expect("generate bmap");
        let bmap = fs::read_to_string(&bmap_path).expect("read bmap");
        assert_eq!(unmapped_journal_blocks(image, &bmap), 0);
        bmap_flash(image, &bmap, &device);
        assert!(journal_on_device_matches_image(image, &device));

        let offset = u64::from(PART_START_LBA) * SECTOR_SIZE;
        run(Command::new("e2fsck")
            .arg("-fn")
            .arg(format!("{image}?offset={offset}")));
    }

    #[test]
    fn read_ext4_journal_rejects_invalid_block_size() {
        let mut sb = [0u8; EXT4_SUPERBLOCK_SIZE];
        let mut set = |o: usize, v: &[u8]| sb[o..o + v.len()].copy_from_slice(v);
        set(EXT4_S_MAGIC, &EXT4_MAGIC.to_le_bytes());
        set(
            EXT4_S_FEATURE_COMPAT,
            &EXT4_FEATURE_COMPAT_HAS_JOURNAL.to_le_bytes(),
        );
        set(EXT4_S_JOURNAL_INUM, &JOURNAL_INODE.to_le_bytes());
        set(EXT4_S_LOG_BLOCK_SIZE, &INVALID_LOG_BLOCK_SIZE.to_le_bytes());

        let image = tempfile::tempfile().expect("create image");
        image
            .write_all_at(&sb, EXT4_SUPERBLOCK_OFFSET)
            .expect("write superblock");
        assert!(read_ext4_journal(&image, 0).is_err());
    }

    #[test]
    fn allocate_ext4_journals_skips_partition_past_image_end() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        create_image_with_mbr(&image, u64::from(PART_START_LBA) * SECTOR_SIZE);

        allocate_ext4_journals(image.to_str().expect("image path")).expect("allocate journals");
    }
}
