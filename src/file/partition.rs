use anyhow::{Context, Result};
use std::fs::File;
use std::io::SeekFrom;
use std::io::Seek;
use std::path::Path;

pub struct PartitionData {
    pub num: u32,
    pub start: u64,
    pub count: u64,
}

pub fn get_partition_data<P: AsRef<Path>>(path: P, partition_num: u32) -> Result<PartitionData> {
    let path = path.as_ref();
    let mut file = File::open(path)
        .with_context(|| format!("failed to open image: {}", path.display()))?;

    // Try GPT first (validates CRC32 — more robust than signature check)
    if let Ok(gpt) = gptman::GPT::find_from(&mut file) {
        let entry = &gpt[partition_num];
        anyhow::ensure!(entry.is_used(), "GPT partition {partition_num} is not used");
        return Ok(PartitionData {
            num: partition_num,
            start: entry.starting_lba,
            count: entry.ending_lba - entry.starting_lba + 1,
        });
    }

    // Try MBR — iter() includes logical partitions (5, 6, ...)
    file.seek(SeekFrom::Start(0))
        .context("failed to seek to start of image")?;
    let mbr = mbrman::MBR::read_from(&mut file, 512)
        .context("image is neither valid GPT nor MBR")?;
    for (i, p) in mbr.iter() {
        if i == partition_num as usize && p.is_used() {
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
    let mut file = File::open(path.as_ref())
        .context("is_gpt: failed to open image")?;
    Ok(gptman::GPT::find_from(&mut file).is_ok())
}
