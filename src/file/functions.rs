use anyhow::{Context, Result};
use log::{debug, warn};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use stdext::function_name;
use uuid::Uuid;

#[derive(clap::ValueEnum, Debug, Clone, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Partition {
    boot,
    rootA,
    cert,
    factory,
}

#[derive(Debug)]
struct PartitionInfo {
    num: String,
    start: String,
    end: String,
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

macro_rules! exec_cmd_with_output {
    ($cmd:expr) => {{
        let res = $cmd
            .output()
            .context(format!("{}: spawn {:?}", function_name!(), $cmd))?;

        let output =
            String::from_utf8(res.stdout).context(format!("{}: get output", function_name!()))?;

        let output = output.trim();

        debug!("{}: {:?}", function_name!(), $cmd);

        output.to_string()
    }};
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
    let mut fdisk = Command::new("fdisk");
    fdisk
        .arg("-l")
        .arg("-o")
        .arg("Device,Start,End")
        .arg(image_file);
    let fdisk_out = exec_cmd_with_output!(fdisk);

    let partition_num = match partition {
        Partition::boot => 1,
        Partition::rootA => 2,
        p @ (Partition::factory | Partition::cert) => {
            let re = Regex::new(r"Disklabel type: (\D{3})").unwrap();

            let matches = re
                .captures(&fdisk_out)
                .context("get_partition_info: regex no matches found")?;
            anyhow::ensure!(
                matches.len() == 2,
                "'get_partition_info: regex contains unexpected number of matches"
            );

            let partition_type = &matches[1];

            debug!("partition type: {partition_type}");

            match (p, partition_type) {
                (Partition::factory, "gpt") => 4,
                (Partition::factory, "dos") => 5,
                (Partition::cert, "gpt") => 5,
                (Partition::cert, "dos") => 6,
                _ => anyhow::bail!("get_partition_info: unhandled partition type"),
            }
        }
    };

    let re = Regex::new(format!(r"{image_file}{partition_num}\s+(\d+)\s+(\d+)").as_str())
        .context("get_partition_info: failed to create regex")?;

    let matches = re
        .captures(&fdisk_out)
        .context("get_partition_info: regex no matches found")?;
    anyhow::ensure!(
        matches.len() == 3,
        "'get_partition_info: regex contains unexpected number of matches"
    );

    let partition_offset = (matches[1].to_string(), matches[2].to_string());

    let info = PartitionInfo {
        num: partition_num.to_string(),
        start: partition_offset.0,
        end: partition_offset.1,
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
        .arg("bs=512")
        .arg(format!("skip={}", partition_info.start))
        .arg(format!("count={}", partition_info.end))
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
    let mut dd = Command::new("dd");
    dd.arg(format!("if={partition_file}"))
        .arg(format!("of={image_file}"))
        .arg("bs=512")
        .arg(format!("seek={}", partition_info.start))
        .arg(format!("count={}", partition_info.end))
        .arg("conv=notrunc,sparse")
        .arg("status=none");
    exec_cmd!(dd);

    let mut fallocate = Command::new("fallocate");
    fallocate.arg("-d").arg(image_file);
    exec_cmd!(fallocate);

    let mut sync = Command::new("sync");
    exec_cmd!(sync);

    Ok(())
}

pub fn generate_bmap_file(image_file: &str) -> Result<()> {
    let mut bmaptool = Command::new("bmaptool");
    bmaptool
        .arg("create")
        .arg("-o")
        .arg(format!("{image_file}.bmap"))
        .arg(image_file);
    exec_cmd!(bmaptool);

    Ok(())
}
