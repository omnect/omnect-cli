use crate::file::{
    COPY_CHUNK_SIZE, IMAGE_BLOCK_SIZE, partition::SECTOR_SIZE, signal::RegisteredTempDir, sparse,
};
use anyhow::{Context, Result};
use log::{debug, warn};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    fs,
    ops::Range,
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};
use stdext::function_name;
use uuid::Uuid;

#[derive(clap::ValueEnum, Debug, Clone, Eq, Hash, PartialEq)]
#[clap(rename_all = "verbatim")]
#[allow(non_camel_case_types)]
pub enum Partition {
    boot,
    rootA,
    cert,
    factory,
}

#[derive(Debug)]
struct PartitionInfo {
    num: u32,
    start: u64,
    count: u64,
}

impl Display for Partition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Partition::boot => write!(f, "boot"),
            Partition::rootA => write!(f, "rootA"),
            Partition::cert => write!(f, "cert"),
            Partition::factory => write!(f, "factory"),
        }
    }
}

impl FromStr for Partition {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Partition> {
        match input {
            "boot" => Ok(Partition::boot),
            "rootA" => Ok(Partition::rootA),
            "cert" => Ok(Partition::cert),
            "factory" => Ok(Partition::factory),
            _ => anyhow::bail!("unknown partition: use either boot, rootA, cert or factory"),
        }
    }
}

// ToDo: find a way to use one implementation "FileCopyParams" instead of "FileCopyToParams" and "FileCopyFromParams"
#[derive(Clone, Debug)]
pub struct FileCopyToParams {
    in_file: std::path::PathBuf,
    partition: Partition,
    out_file: std::path::PathBuf,
}

impl FileCopyToParams {
    pub fn new(
        in_file: &std::path::Path,
        partition: Partition,
        out_file: &std::path::Path,
    ) -> Self {
        FileCopyToParams {
            in_file: in_file.to_path_buf(),
            partition,
            out_file: out_file.to_path_buf(),
        }
    }
}

impl FromStr for FileCopyToParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let err_msg = "format not matched: in-file-path,out-partition:out-file-path";

        anyhow::ensure!(
            s.matches(',').count() == 1 && s.matches(':').count() == 1,
            err_msg
        );

        let v: Vec<&str> = s.split(&[',', ':']).collect();

        anyhow::ensure!(v.len() == 3, err_msg);

        let in_file = std::path::PathBuf::from(v[0]);
        let partition = Partition::from_str(v[1])?;
        let out_file = std::path::PathBuf::from(v[2]);

        anyhow::ensure!(
            in_file.try_exists().is_ok_and(|exists| exists),
            "in-file-path doesn't exist"
        );
        anyhow::ensure!(
            out_file.is_absolute(),
            "out-file-path isn't an absolute path"
        );

        Ok(Self {
            in_file,
            partition,
            out_file,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FileCopyFromParams {
    in_file: std::path::PathBuf,
    partition: Partition,
    out_file: std::path::PathBuf,
}

impl FileCopyFromParams {
    pub fn new(
        in_file: &std::path::Path,
        partition: Partition,
        out_file: &std::path::Path,
    ) -> Self {
        FileCopyFromParams {
            in_file: in_file.to_path_buf(),
            partition,
            out_file: out_file.to_path_buf(),
        }
    }
}

impl FromStr for FileCopyFromParams {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let err_msg = "format not matched: in-partition:in-file-path,out-file-path";

        anyhow::ensure!(
            s.matches(',').count() == 1 && s.matches(':').count() == 1,
            err_msg
        );

        let v: Vec<&str> = s.split(&[',', ':']).collect();

        anyhow::ensure!(v.len() == 3, err_msg);

        let partition = Partition::from_str(v[0])?;
        let in_file = std::path::PathBuf::from(v[1]);
        let out_file = std::path::PathBuf::from(v[2]);

        Ok(Self {
            in_file,
            partition,
            out_file,
        })
    }
}

macro_rules! exec_cmd {
    ($cmd:ident) => {
        anyhow::ensure!(
            $cmd.status()
                .context(format!("{}: status failed: {:?}", function_name!(), $cmd))?
                .success(),
            format!("{}: cmd failed: {:?}", function_name!(), $cmd)
        );
        debug!("{}: {:?}", function_name!(), $cmd);
    };
}

macro_rules! try_exec_cmd {
    ($cmd:ident) => {
        if $cmd
            .status()
            .context(format!("{}: status failed: {:?}", function_name!(), $cmd))?
            .success()
        {
            debug!("{}: {:?}", function_name!(), $cmd);
        } else {
            warn!("{}: {:?}", function_name!(), $cmd)
        }
    };
}

pub fn copy_to_image(file_copy_params: &[FileCopyToParams], image_file: &Path) -> Result<()> {
    // the partition files get their own dir, so they never collide with the image
    let working_dir = RegisteredTempDir::new(
        tempfile::tempdir().context("copy_to_image: cannot create a temp dir")?,
    );
    let image_file = image_file.to_str().unwrap();
    let mut partition_map: HashMap<&Partition, Vec<(&PathBuf, &PathBuf)>> = HashMap::new();

    // create map with partition as key
    for params in file_copy_params.iter() {
        let e = (&params.in_file, &params.out_file);
        partition_map
            .entry(&params.partition)
            .and_modify(|v| v.push(e))
            .or_insert(vec![e]);
    }

    // 1. for each involved partition
    for partition in partition_map.keys() {
        let mut partition_file = working_dir.path().to_path_buf();
        let partition_info = get_partition_info(image_file, partition)?;

        partition_file.push(Path::new(&format!("{}.img", partition_info.num)));
        let partition_file = partition_file.to_str().unwrap();

        // 2. read partition
        read_partition(image_file, partition_file, &partition_info)?;

        // 3. copy files
        for (in_file, out_file) in partition_map.get(partition).unwrap().iter() {
            let dir_path = out_file.parent().context(format!(
                "copy_to_image: invalid destination path {}",
                out_file.to_str().unwrap()
            ))?;

            let out_file = out_file.to_str().unwrap();

            if **partition == Partition::boot {
                let mut p = PathBuf::from("/");

                for dir in dir_path.iter().skip(1).map(|d| d.to_str().unwrap()) {
                    p.push(dir);
                    let mut mmd = Command::new("mmd");
                    mmd.arg("-D")
                        .arg("sS")
                        .arg("-i")
                        .arg(partition_file)
                        .arg(p.to_str().unwrap());
                    // we ignore `mmd` errors in order to ignore potential name clashes when a dir already exists
                    // in case mmd fails mcopy will fail respectively with a reasonable error output
                    try_exec_cmd!(mmd);
                }

                let mut mcopy = Command::new("mcopy");
                mcopy
                    .arg("-o")
                    .arg("-i")
                    .arg(partition_file)
                    .arg(in_file)
                    .arg(format!("::{out_file}"));
                exec_cmd!(mcopy);
            } else {
                let mut e2mkdir = Command::new("e2mkdir");
                e2mkdir.arg(format!("{partition_file}:{}", dir_path.to_str().unwrap()));
                exec_cmd!(e2mkdir);

                let mut e2cp = Command::new("e2cp");
                e2cp.arg(in_file)
                    .arg(format!("{partition_file}:{out_file}"));
                exec_cmd!(e2cp);
            }
        }

        // 4. write back partition
        write_partition(image_file, partition_file, &partition_info)?;
    }

    Ok(())
}

pub fn copy_from_image(file_copy_params: &[FileCopyFromParams], image_file: &Path) -> Result<()> {
    // the partition files get their own dir, so they never collide with the image
    let working_dir = RegisteredTempDir::new(
        tempfile::tempdir().context("copy_from_image: cannot create a temp dir")?,
    );
    let image_file = image_file.to_str().unwrap();

    for param in file_copy_params.iter() {
        let mut partition_file = working_dir.path().to_path_buf();

        let partition_info = get_partition_info(image_file, &param.partition)?;
        let in_file = param.in_file.to_str().unwrap();

        partition_file.push(Path::new(&format!("{}.img", partition_info.num)));
        let partition_file = partition_file.to_str().unwrap();

        read_partition(image_file, partition_file, &partition_info)?;

        anyhow::ensure!(
            param
                .out_file
                .parent()
                .unwrap()
                .try_exists()
                .is_ok_and(|exists| exists),
            "copy_from_image: output dir does not exist."
        );

        // copy
        if param.partition == Partition::boot {
            let mut tmp_out_file = working_dir.path().to_path_buf();
            // mcopy deadlocks when target file is not residing in workingdir so we copy to a temp file
            tmp_out_file.push(format!(
                "{}-{}",
                Uuid::new_v4(),
                param.out_file.file_name().unwrap().to_str().unwrap()
            ));

            let mut mcopy = Command::new("mcopy");
            mcopy
                .arg("-o")
                .arg("-i")
                .arg(partition_file)
                .arg(format!("::{in_file}"))
                .arg(&tmp_out_file);
            exec_cmd!(mcopy);
            // instead of rename we copy and delete to prevent "Invalid cross-device link" errors
            let bytes_copied = fs::copy(&tmp_out_file, &param.out_file).context(format!(
                "copy_from_image: couldn't copy temp file {} to destination {}",
                tmp_out_file.to_str().unwrap(),
                param.out_file.to_str().unwrap()
            ))?;
            anyhow::ensure!(
                tmp_out_file.metadata().unwrap().len() == bytes_copied,
                "copy_from_image: copy temp file failed"
            );
            fs::remove_file(&tmp_out_file).context(format!(
                "copy_from_image: couldn't delete temp file {}",
                tmp_out_file.to_str().unwrap()
            ))?;
        } else {
            let mut e2cp = Command::new("e2cp");
            e2cp.arg(format!("{partition_file}:{in_file}"))
                .arg(param.out_file.to_str().unwrap());
            exec_cmd!(e2cp);
            // since e2cp doesn't return errors in any case we check if output file exists
            anyhow::ensure!(
                param.out_file.try_exists().is_ok_and(|exists| exists),
                format!("copy_from_image: cmd failed: {:?}", e2cp)
            )
        }
    }

    Ok(())
}

pub fn read_file_from_image(
    path: impl AsRef<Path>,
    partition: Partition,
    image_file: impl AsRef<Path>,
) -> Result<String> {
    let tmp_file = tempfile::NamedTempFile::new()
        .context("read_file_from_image: could not create temporary file path")?;

    let params = FileCopyFromParams::new(path.as_ref(), partition, tmp_file.path());

    copy_from_image(&[params], image_file.as_ref())
        .context("read_file_from_image: could not copy file content")?;

    let content = std::fs::read_to_string(tmp_file.path())
        .context("read_file_from_image: could not read file content")?;

    Ok(content)
}

fn get_partition_info(image_file: &str, partition: &Partition) -> Result<PartitionInfo> {
    use crate::file::partition::{get_partition_data, is_gpt};

    let gpt =
        is_gpt(image_file).context("get_partition_info: failed to detect partition table type")?;

    let partition_num: u32 = match partition {
        Partition::boot => 1,
        Partition::rootA => 2,
        Partition::factory => {
            if gpt {
                4
            } else {
                5
            }
        }
        Partition::cert => {
            if gpt {
                5
            } else {
                6
            }
        }
    };

    debug!("get_partition_info: partition={partition}, num={partition_num}, gpt={gpt}");

    let data = get_partition_data(image_file, partition_num)
        .with_context(|| format!("get_partition_info: failed to read partition {partition_num}"))?;

    let info = PartitionInfo {
        num: data.num,
        start: data.start,
        count: data.count,
    };

    debug!("get_partition_info: {:?}", info);

    Ok(info)
}

fn read_partition(
    image_file: &str,
    partition_file: &str,
    partition_info: &PartitionInfo,
) -> Result<()> {
    if let Ok(true) = PathBuf::from(partition_file).try_exists() {
        return Ok(());
    }

    let image =
        fs::File::open(image_file).context(format!("read_partition: cannot open {image_file}"))?;

    sparse::copy_to_new_file(
        &image,
        partition_info.start * SECTOR_SIZE,
        Path::new(partition_file),
        partition_info.count * SECTOR_SIZE,
    )
}

fn write_partition(
    image_file: &str,
    partition_file: &str,
    partition_info: &PartitionInfo,
) -> Result<()> {
    let partition = fs::File::open(partition_file)
        .context(format!("write_partition: cannot open {partition_file}"))?;
    let image = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(image_file)
        .context(format!("write_partition: cannot open {image_file}"))?;

    let image_offset = partition_info.start * SECTOR_SIZE;
    let len = partition
        .metadata()
        .context("write_partition: cannot get partition file size")?
        .len()
        .min(partition_info.count * SECTOR_SIZE);

    // Only write blocks that differ: unchanged blocks keep their allocation, so a bmap
    // created from the result still maps every block the source image mapped. A zero block
    // that a tool wrote into a hole is data in the partition file and must become data in the
    // image too, else a bmap flash keeps the old content there.
    let partition_data = sparse::data_ranges(&partition, 0, len)?;
    let image_data = sparse::data_ranges(&image, image_offset, image_offset + len)?;
    // a block that is a hole in both files is zero in both, so it never needs a write
    let block_size = IMAGE_BLOCK_SIZE as u64;
    let spans = sparse::merge(
        partition_data
            .iter()
            .cloned()
            .chain(
                image_data
                    .iter()
                    .map(|r| r.start - image_offset..r.end - image_offset),
            )
            .map(|r| r.start / block_size * block_size..r.end.next_multiple_of(block_size).min(len))
            .collect(),
    );
    let mut src = vec![0u8; COPY_CHUNK_SIZE];
    let mut dst = vec![0u8; COPY_CHUNK_SIZE];
    for span in spans {
        write_changed_blocks(
            &partition,
            &image,
            image_offset,
            span,
            (&partition_data, &image_data),
            (&mut src, &mut dst),
        )?;
    }

    Ok(())
}

fn write_changed_blocks(
    partition: &fs::File,
    image: &fs::File,
    image_offset: u64,
    span: Range<u64>,
    (partition_data, image_data): (&[Range<u64>], &[Range<u64>]),
    (src, dst): (&mut [u8], &mut [u8]),
) -> Result<()> {
    let mut pos = span.start;
    while pos < span.end {
        let n = usize::try_from((span.end - pos).min(COPY_CHUNK_SIZE as u64))?;
        partition
            .read_exact_at(&mut src[..n], pos)
            .context("write_partition: cannot read partition file")?;
        image
            .read_exact_at(&mut dst[..n], image_offset + pos)
            .context("write_partition: cannot read image")?;

        let blocks = src[..n]
            .chunks(IMAGE_BLOCK_SIZE)
            .zip(dst[..n].chunks(IMAGE_BLOCK_SIZE));
        for (i, (s, d)) in blocks.enumerate() {
            let block_pos = pos + (i * IMAGE_BLOCK_SIZE) as u64;
            let block = block_pos..block_pos + s.len() as u64;
            let image_block = image_offset + block.start..image_offset + block.end;
            if s != d || (has_data(partition_data, &block) && has_hole(image_data, &image_block)) {
                image
                    .write_all_at(s, image_block.start)
                    .context("write_partition: cannot write image")?;
            }
        }
        pos += n as u64;
    }

    Ok(())
}

fn has_data(data_ranges: &[Range<u64>], block: &Range<u64>) -> bool {
    let i = data_ranges.partition_point(|r| r.end <= block.start);
    data_ranges.get(i).is_some_and(|r| r.start < block.end)
}

fn has_hole(data_ranges: &[Range<u64>], block: &Range<u64>) -> bool {
    let i = data_ranges.partition_point(|r| r.end <= block.start);
    !data_ranges
        .get(i)
        .is_some_and(|r| r.start <= block.start && block.end <= r.end)
}

#[cfg(test)]
mod tests {
    use crate::file::functions::*;
    use std::os::unix::fs::MetadataExt;

    const PART_NUM: u32 = 1;
    const PART_START: u64 = 8;
    const PART_SECTORS: u64 = 64;
    const OLD_CONTENT_BYTE: u8 = 0xa5;
    const ZEROED_BLOCK: usize = 2;
    // the first partition of an old MBR layout
    const UNALIGNED_PART_START: u64 = 63;

    fn partition_info() -> PartitionInfo {
        PartitionInfo {
            num: PART_NUM,
            start: PART_START,
            count: PART_SECTORS,
        }
    }

    #[test]
    fn read_partition_fails_for_partition_past_the_image_end() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        let partition = dir.path().join("partition.img");
        fs::File::create(&image)
            .expect("create image")
            .set_len((PART_START + PART_SECTORS - 1) * SECTOR_SIZE)
            .expect("set image size");

        assert!(
            read_partition(
                image.to_str().expect("image path"),
                partition.to_str().expect("partition path"),
                &partition_info(),
            )
            .is_err()
        );
    }

    #[test]
    fn write_partition_writes_block_changed_to_zeros() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        let partition = dir.path().join("partition.img");
        let image_len = usize::try_from((PART_START + PART_SECTORS) * SECTOR_SIZE).expect("size");
        let part_offset = usize::try_from(PART_START * SECTOR_SIZE).expect("offset");

        fs::write(&image, vec![OLD_CONTENT_BYTE; image_len]).expect("write image");
        let mut content = vec![OLD_CONTENT_BYTE; image_len - part_offset];
        content[ZEROED_BLOCK * IMAGE_BLOCK_SIZE..(ZEROED_BLOCK + 1) * IMAGE_BLOCK_SIZE].fill(0);
        fs::write(&partition, &content).expect("write partition");

        write_partition(
            image.to_str().expect("image path"),
            partition.to_str().expect("partition path"),
            &partition_info(),
        )
        .expect("write partition");

        let written = fs::read(&image).expect("read image");
        assert!(written[part_offset..] == content[..], "partition differs");
    }

    #[test]
    fn write_partition_keeps_holes_of_unchanged_blocks() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        let partition = dir.path().join("partition.img");
        fs::File::create(&image)
            .expect("create image")
            .set_len((PART_START + PART_SECTORS) * SECTOR_SIZE)
            .expect("set image size");
        fs::File::create(&partition)
            .expect("create partition")
            .set_len(PART_SECTORS * SECTOR_SIZE)
            .expect("set partition size");
        let allocated = || fs::metadata(&image).expect("image metadata").blocks();
        let before = allocated();

        write_partition(
            image.to_str().expect("image path"),
            partition.to_str().expect("partition path"),
            &partition_info(),
        )
        .expect("write partition");

        assert_eq!(allocated(), before, "unchanged zero blocks were written");
    }

    #[test]
    fn write_partition_maps_zero_block_of_an_unaligned_partition() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        let partition = dir.path().join("partition.img");
        let info = PartitionInfo {
            num: PART_NUM,
            start: UNALIGNED_PART_START,
            count: PART_SECTORS,
        };
        let image_file = fs::File::create(&image).expect("create image");
        image_file
            .set_len((UNALIGNED_PART_START + PART_SECTORS) * SECTOR_SIZE)
            .expect("set image size");
        let partition_file = fs::File::create(&partition).expect("create partition");
        partition_file
            .set_len(PART_SECTORS * SECTOR_SIZE)
            .expect("set partition size");
        let block = (ZEROED_BLOCK * IMAGE_BLOCK_SIZE) as u64;
        partition_file
            .write_all_at(&[0; IMAGE_BLOCK_SIZE], block)
            .expect("write zero block");

        write_partition(
            image.to_str().expect("image path"),
            partition.to_str().expect("partition path"),
            &info,
        )
        .expect("write partition");

        let block = UNALIGNED_PART_START * SECTOR_SIZE + block;
        let block = block..block + IMAGE_BLOCK_SIZE as u64;
        let image_data =
            sparse::data_ranges(&image_file, block.start, block.end).expect("data ranges");
        assert_eq!(image_data, vec![block], "zero block is still a hole");
    }

    #[test]
    fn write_partition_maps_zero_block_written_into_a_hole() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.wic");
        let partition = dir.path().join("partition.img");
        let image_file = fs::File::create(&image).expect("create image");
        image_file
            .set_len((PART_START + PART_SECTORS) * SECTOR_SIZE)
            .expect("set image size");
        let partition_file = fs::File::create(&partition).expect("create partition");
        partition_file
            .set_len(PART_SECTORS * SECTOR_SIZE)
            .expect("set partition size");
        let block = (ZEROED_BLOCK * IMAGE_BLOCK_SIZE) as u64;
        partition_file
            .write_all_at(&[0; IMAGE_BLOCK_SIZE], block)
            .expect("write zero block");

        write_partition(
            image.to_str().expect("image path"),
            partition.to_str().expect("partition path"),
            &partition_info(),
        )
        .expect("write partition");

        let block = PART_START * SECTOR_SIZE + block;
        assert_eq!(
            sparse::data_ranges(
                &image_file,
                0,
                image_file.metadata().expect("metadata").len()
            )
            .expect("data ranges"),
            vec![block..block + IMAGE_BLOCK_SIZE as u64]
        );
    }
}
