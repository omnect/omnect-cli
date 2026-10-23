use crate::file::partition::SECTOR_SIZE;
use anyhow::{Context, Result};
use log::{debug, warn};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use stdext::function_name;
use uuid::Uuid;

const COMPARE_BLOCK_SIZE: usize = 4096;
const COPY_CHUNK_SIZE: usize = 1024 * 1024;

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
    // we use the folder the image is located in
    // the caller is responsible to create a /tmp/ directory if needed
    let working_dir = image_file
        .parent()
        .context("copy_to_image: cannot get directory of image")?
        .to_path_buf();
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
        let mut partition_file = working_dir.clone();
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
    // we use the folder the image is located in
    // the caller is responsible to create a /tmp/ directory if needed
    let working_dir = image_file
        .parent()
        .context("copy_to_image: cannot get directory of image")?
        .to_path_buf();
    let image_file = image_file.to_str().unwrap();

    for param in file_copy_params.iter() {
        let mut partition_file = working_dir.clone();

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
            let mut tmp_out_file = working_dir.clone();
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

    let mut dd = Command::new("dd");
    dd.arg(format!("if={image_file}"))
        .arg(format!("of={partition_file}"))
        .arg(format!("bs={SECTOR_SIZE}"))
        .arg(format!("skip={}", partition_info.start))
        .arg(format!("count={}", partition_info.count))
        .arg("conv=sparse")
        .arg("status=none");
    exec_cmd!(dd);

    let mut sync = Command::new("sync");
    exec_cmd!(sync);

    Ok(())
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
    // created from the result still maps every block the source image mapped.
    let mut src = vec![0u8; COPY_CHUNK_SIZE];
    let mut dst = vec![0u8; COPY_CHUNK_SIZE];
    let mut pos = 0;
    while pos < len {
        let n = usize::try_from((len - pos).min(COPY_CHUNK_SIZE as u64))?;
        partition
            .read_exact_at(&mut src[..n], pos)
            .context("write_partition: cannot read partition file")?;
        image
            .read_exact_at(&mut dst[..n], image_offset + pos)
            .context("write_partition: cannot read image")?;

        let blocks = src[..n]
            .chunks(COMPARE_BLOCK_SIZE)
            .zip(dst[..n].chunks(COMPARE_BLOCK_SIZE));
        for (i, (s, d)) in blocks.enumerate() {
            if s != d {
                image
                    .write_all_at(s, image_offset + pos + (i * COMPARE_BLOCK_SIZE) as u64)
                    .context("write_partition: cannot write image")?;
            }
        }
        pos += n as u64;
    }

    image
        .sync_all()
        .context("write_partition: cannot sync image")
}
