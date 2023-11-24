use anyhow::{Context, Result};
use log::debug;
use std::collections::HashMap;
use std::fmt::format;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use uuid::Uuid;

#[derive(clap::ValueEnum, Debug, Clone, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Partition {
    boot,
    rootA,
    cert,
    factory,
}

pub fn copy_to_image(
    file: &PathBuf,
    image_file: &PathBuf,
    partition: Partition,
    destination: String,
) -> Result<()> {
    let partition_map = get_partition_type(image_file)?;

    let uuid = Uuid::new_v4();
    let tmp_dir = PathBuf::from(format!("/tmp/{uuid}"));
    fs::create_dir_all(&tmp_dir).context("copy_to_image: create {tmp_dir}")?;

    read_in_partition(image_file, tmp_dir, partition_map[&partition])?;
    Ok(())
    /*
     get_partition_type
    read_in_partition

    # copy file
    if [ "${p}" != "boot" ]; then
        d_echo "e2cp ${i} /tmp/${uuid}/${p}.img:${o}"
        e2mkdir /tmp/${uuid}/${p}.img:$(dirname ${o})
        e2cp ${i} /tmp/${uuid}/${p}.img:${o}
    else
        lastdir=""
        IFS='/' read -ra path <<< $(dirname ${o})
        for dir in "${path[@]}"; do
            if [ ! -z "$dir" ]; then
                dir="${lastdir}"/"${dir}"
                # we ignore errors in order to ignore potential name clashes here
                # in case mmd fails mcopy will fail respectivly with a reasonable error putput
                d_echo "mmd -D sS -i /tmp/""${uuid}""/""${p}"".img ::""${dir}"""
                mmd -D sS -i /tmp/"${uuid}"/"${p}".img ::"${dir}" || true
                lastdir="$dir"
            fi
        done

        d_echo "mcopy -o -i /tmp/${uuid}/${p}.img ${i} ::${o}"
        mcopy -o -i /tmp/"${uuid}"/"${p}".img "${i}" ::"${o}"
    fi

    write_back_partition */
}

fn get_partition_type(image_file: &PathBuf) -> Result<HashMap<Partition, u8>> {
    let fdisk = Command::new("fdisk")
        .arg("-l")
        .arg(image_file)
        .stdout(Stdio::piped())
        .spawn()
        .context("get_partition_type: spawn fdisk")?;

    let fdisk_out = fdisk
        .stdout
        .context("get_partition_type: spawn fdisk output")?;

    let grep = Command::new("grep")
        .arg("^Disklabel type:")
        .stdin(Stdio::from(fdisk_out))
        .stdout(Stdio::piped())
        .spawn()
        .context("get_partition_type: spawn grep")?;

    let grep_out = grep
        .stdout
        .context("get_partition_type: spawn grep output")?;

    let awk = Command::new("awk")
        .arg("{print $NF}")
        .stdin(Stdio::from(grep_out))
        .stdout(Stdio::piped())
        .spawn()
        .context("get_partition_type: spawn awk")?;

    let output = awk
        .wait_with_output()
        .context("get_partition_type: spawn awk output")?;

    let partition_type =
        String::from_utf8(output.stdout).context("get_partition_type: get output")?;

    let partition_type = partition_type.trim();

    debug!("partition type: {partition_type}");

    let (factory, cert) = match partition_type {
        "gpt" => (4u8, 5u8),
        "dos" => (5u8, 6u8),
        _ => anyhow::bail!("get_partition_type: unhandled partition type"),
    };

    Ok(HashMap::from([
        (Partition::boot, 1),
        (Partition::rootA, 2),
        (Partition::factory, factory),
        (Partition::cert, cert),
    ]))
}

fn read_in_partition(image_file: &PathBuf, mut tmp_dir: PathBuf, partition: u8) -> Result<()> {
    let fdisk = Command::new("fdisk")
        .arg("-l")
        .arg("-o")
        .arg("Device,Start,End")
        .arg(image_file)
        .stdout(Stdio::piped())
        .spawn()
        .context("read_in_partition: spawn fdisk")?;

    let fdisk_out = fdisk
        .stdout
        .context("read_in_partition: spawn fdisk output")?;

    let grep = Command::new("grep")
        .arg(format!("{}{partition}", image_file.to_str().unwrap()))
        .stdin(Stdio::from(fdisk_out))
        .stdout(Stdio::piped())
        .spawn()
        .context("read_in_partition: spawn grep")?;

    let grep_out = grep
        .stdout
        .context("read_in_partition: spawn grep output")?;

    let awk = Command::new("awk")
        .arg("{print $2 \" \" $3}")
        .stdin(Stdio::from(grep_out))
        .stdout(Stdio::piped())
        .spawn()
        .context("read_in_partition: spawn awk")?;

    let output = awk
        .wait_with_output()
        .context("read_in_partition: spawn awk output")?;

    let partition_offset =
        String::from_utf8(output.stdout).context("read_in_partition: get output")?;

    let partition_offset = partition_offset.trim();

    let partition_offset = partition_offset
        .split_once(" ")
        .context("read_in_partition: split offset")?;

    debug!("start: {} end: {}", partition_offset.0, partition_offset.1);

    tmp_dir.push(Path::new(&partition.to_string()));

    let status = Command::new("dd")
        .arg(format!("if={}", image_file.to_str().unwrap()))
        .arg(format!("of={}.img", tmp_dir.to_str().unwrap()))
        .arg("bs=512")
        .arg(format!("skip={}", partition_offset.0))
        .arg(format!("count={}", partition_offset.1))
        .arg("conv=sparse")
        .arg("status=none")
        .status()
        .context("read_in_partition: dd status failed")?;

    anyhow::ensure!(status.success(), "read_in_partition: dd failed");

    Ok(())

    /*     d_echo read_in_partition ${p}
    part_offset=($(fdisk -l -o Device,Start,End ${w} | grep ${w}${!p} | awk '{print $2 " " $3}'))
    dd if=${w} of=/tmp/${uuid}/${p}.img bs=512 skip=${part_offset[0]} count=${part_offset[1]} conv=sparse status=none
    sync */
}

/*
function write_back_partition() {
    d_echo write_back_partition ${p}
    d_echo "dd if=/tmp/${uuid}/${p}.img of=${w} bs=512 seek=${part_offset[0]} count=${part_offset[1]} conv=notrunc,sparse status=none"
    dd if=/tmp/${uuid}/${p}.img of=${w} bs=512 seek=${part_offset[0]} count=${part_offset[1]} conv=notrunc,sparse status=none
    # there are cases where dd with conv=notrunc,sparse is not sufficient:
    # e.g. the original file lies in a ramdisk
    fallocate -d ${w}
    sync
}

function read_in_rootA() {
    d_echo read_in_rootA
    local _part_offset=($(fdisk -l ${w} | grep ${w}${rootA} | awk '{print $2 " " $4}'))
    d_echo "dd if=${w} of=/tmp/${uuid}/rootA.img bs=512 skip=${_part_offset[0]} count=${_part_offset[1]} conv=sparse status=none"
    dd if=${w} of=/tmp/${uuid}/rootA.img bs=512 skip=${_part_offset[0]} count=${_part_offset[1]} conv=sparse status=none
    sync
}

function config_hostname () {
    d_echo config_hostname
    hostname=$(grep "^hostname" ${1})

    # possibly remove inline comments
    hostname=${hostname%%\#*}

    hostname=$(echo ${hostname} | cut -d "=" -f2 | xargs)

    if [ -z "${hostname}" ]; then
        error "hostname is empty"
        exit 1
    fi

    echo "${hostname}" > /tmp/${uuid}/hostname

    read_in_rootA
    e2cp /tmp/${uuid}/rootA.img:/etc/hosts /tmp/${uuid}/hosts
    sed -i "s/^127.0.1.1\(.*\)/127.0.1.1 ${hostname}/" /tmp/${uuid}/hosts

    if [ "${p}" != "factory" ]; then
        error "can not configure hostname"
        exit 1
    fi

    e2mkdir /tmp/${uuid}/${p}.img:/etc
    e2cp /tmp/${uuid}/hostname /tmp/${uuid}/${p}.img:/etc/hostname
    e2cp /tmp/${uuid}/hosts /tmp/${uuid}/${p}.img:/etc/hosts
}

function copy_identity_config() {
    if [ "${p}" != "factory" ]; then
        error "can not copy identity config"
        exit 1
    fi
    d_echo e2cp ${c} /tmp/${uuid}/${p}.img:/etc/aziot/config.toml
    e2mkdir /tmp/${uuid}/${p}.img:/etc/aziot
    e2cp ${c} /tmp/${uuid}/${p}.img:/etc/aziot/config.toml
    config_hostname ${c}
}
*/
