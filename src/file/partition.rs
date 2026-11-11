use anyhow::{Context, Result};
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;

pub const SECTOR_SIZE: u64 = 512;

#[derive(Debug)]
pub struct PartitionData {
    pub num: u32,
    pub start: u64,
    pub count: u64,
}

/// Fails if the partition does not fit into the image, so a corrupt partition table cannot make
/// a read or write go to another offset.
pub fn get_partition_data<P: AsRef<Path>>(path: P, partition_num: u32) -> Result<PartitionData> {
    let path = path.as_ref();
    let mut file =
        File::open(path).with_context(|| format!("failed to open image: {}", path.display()))?;
    let data = find_partition(&mut file, partition_num)?;
    let image_len = file
        .metadata()
        .context("failed to get the image size")?
        .len();
    let end = data
        .start
        .checked_add(data.count)
        .and_then(|end| end.checked_mul(SECTOR_SIZE));
    anyhow::ensure!(
        end.is_some_and(|end| end <= image_len),
        "partition {partition_num} ends after the end of the image"
    );

    Ok(data)
}

fn find_partition(file: &mut File, partition_num: u32) -> Result<PartitionData> {
    // Try GPT first (validates CRC32 — more robust than signature check).
    // Capture any error so we can attach it as context if MBR also fails.
    let gpt_err = match gptman::GPT::find_from(file) {
        Ok(gpt) => {
            let entry = gpt
                .iter()
                .find(|(i, _)| *i == partition_num)
                .map(|(_, e)| e)
                .with_context(|| format!("GPT partition {partition_num} out of range"))?;
            anyhow::ensure!(entry.is_used(), "GPT partition {partition_num} is not used");
            anyhow::ensure!(
                entry.ending_lba >= entry.starting_lba,
                "GPT partition {partition_num} has invalid LBA range (ending_lba < starting_lba)"
            );
            // the callers count in SECTOR_SIZE units
            let scale = gpt.sector_size / SECTOR_SIZE;
            let sectors = |lba: u64| {
                lba.checked_mul(scale)
                    .with_context(|| format!("GPT partition {partition_num} is too large"))
            };
            return Ok(PartitionData {
                num: partition_num,
                start: sectors(entry.starting_lba)?,
                count: sectors(
                    (entry.ending_lba - entry.starting_lba)
                        .checked_add(1)
                        .with_context(|| format!("GPT partition {partition_num} is too large"))?,
                )?,
            });
        }
        Err(e) => e,
    };

    // Try MBR — iter() includes logical partitions (5, 6, ...)
    file.seek(SeekFrom::Start(0))
        .context("failed to seek to start of image")?;
    let mbr = mbrman::MBR::read_from(file, SECTOR_SIZE as u32)
        .with_context(|| format!("image is neither valid GPT nor MBR (GPT error: {gpt_err})"))?;
    for (i, p) in mbr.iter() {
        if u32::try_from(i).ok() == Some(partition_num) && p.is_used() {
            return Ok(PartitionData {
                num: partition_num,
                start: p.starting_lba as u64,
                count: p.sectors as u64,
            });
        }
    }
    anyhow::bail!("partition {partition_num} not found in image")
}

pub fn is_gpt<P: AsRef<Path>>(path: P) -> Result<bool> {
    let mut file = File::open(path.as_ref()).context("is_gpt: failed to open image")?;
    Ok(gptman::GPT::find_from(&mut file).is_ok())
}

#[cfg(test)]
mod tests {
    use crate::file::partition::*;
    use gptman::{GPT, GPTPartitionEntry};
    use std::path::PathBuf;

    const GPT_SECTOR_SIZE: u64 = 4096;
    const DISK_SECTORS: u64 = 64;
    const PART_NUM: u32 = 1;
    const OTHER_PART_NUM: u32 = 2;
    const PART_SECTORS: u64 = 8;
    const GUID: [u8; 16] = [0xff; 16];
    // times 8 for 4K sectors and times 512 bytes is more than u64::MAX
    const OVERFLOWING_LBA: u64 = 1 << 55;

    /// A 4K-sector GPT image with partition `PART_NUM` at `first_usable_lba`.
    fn gpt_image(dir: &Path) -> (PathBuf, u64) {
        let path = dir.join("image.wic");
        let mut file = File::create(&path).expect("create image");
        file.set_len(DISK_SECTORS * GPT_SECTOR_SIZE)
            .expect("set image size");
        let mut gpt = GPT::new_from(&mut file, GPT_SECTOR_SIZE, GUID).expect("create gpt");
        let start = gpt.header.first_usable_lba;
        gpt[PART_NUM] = GPTPartitionEntry {
            partition_type_guid: GUID,
            unique_partition_guid: GUID,
            starting_lba: start,
            ending_lba: start + PART_SECTORS - 1,
            attribute_bits: 0,
            partition_name: "part".into(),
        };
        gpt.write_into(&mut file).expect("write gpt");
        (path, start)
    }

    #[test]
    fn get_partition_data_converts_4k_gpt_sectors() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let (path, start) = gpt_image(dir.path());

        let data = get_partition_data(&path, PART_NUM).expect("partition data");
        let scale = GPT_SECTOR_SIZE / SECTOR_SIZE;
        assert_eq!(data.start, start * scale);
        assert_eq!(data.count, PART_SECTORS * scale);
    }

    #[test]
    fn get_partition_data_rejects_a_partition_after_the_image_end() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let (path, _) = gpt_image(dir.path());
        let file = File::options().write(true).open(&path).expect("open image");
        file.set_len(PART_SECTORS * GPT_SECTOR_SIZE)
            .expect("set image size");

        let error = get_partition_data(&path, PART_NUM).expect_err("partition after the end");
        assert!(
            error
                .to_string()
                .contains("ends after the end of the image")
        );
    }

    /// Writes partition `num` with the LBA range `starting_lba..=ending_lba`, which may lie outside
    /// the disk, into the image at `path`.
    fn write_lba_range(path: &Path, num: u32, starting_lba: u64, ending_lba: u64) {
        // `GPT::write_into` refuses such an entry, the primary header and entries do not check it
        let mut file = File::options()
            .read(true)
            .write(true)
            .open(path)
            .expect("open image");
        let mut gpt = GPT::find_from(&mut file).expect("read gpt");
        gpt[num] = GPTPartitionEntry {
            starting_lba,
            ending_lba,
            ..gpt[PART_NUM].clone()
        };
        let partitions: Vec<_> = gpt.iter().map(|(_, entry)| entry.clone()).collect();
        gpt.header
            .write_into(&mut file, GPT_SECTOR_SIZE, &partitions)
            .expect("write gpt header");
    }

    #[test]
    fn get_partition_data_rejects_an_overflowing_offset() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let (path, _) = gpt_image(dir.path());
        write_lba_range(
            &path,
            PART_NUM,
            OVERFLOWING_LBA,
            OVERFLOWING_LBA + PART_SECTORS - 1,
        );

        let error = get_partition_data(&path, PART_NUM).expect_err("overflowing offset");
        assert!(
            error
                .to_string()
                .contains("ends after the end of the image")
        );
    }

    #[test]
    fn get_partition_data_rejects_an_overflowing_sector_count() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let (path, _) = gpt_image(dir.path());
        // gptman cannot read a GPT whose used partitions all start at LBA 0, so `PART_NUM` stays
        write_lba_range(&path, OTHER_PART_NUM, 0, u64::MAX);

        let error = get_partition_data(&path, OTHER_PART_NUM).expect_err("overflowing count");
        assert!(error.to_string().contains("is too large"));
    }
}
