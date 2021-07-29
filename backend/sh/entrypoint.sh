#!/bin/bash

# restore saved permissions for error file
function finish {
    chown ${uid}:${gid} ${err_file_path}
}
trap finish EXIT

function check_mount() {
    tmp_mount=/tmp/check_mount
    mkdir -p ${tmp_mount}
    if [ $(mount -t devtmpfs none ${tmp_mount} &> /dev/null;echo $?) != "0" ]; then
        echo "error: container has no mount privileges" 1>&2
        exit 1
    fi
    umount ${tmp_mount}
    rmdir ${tmp_mount}
}

function devtmpfs_mount() {
    tmp_dir=/tmp/devtmpfs_mount
    mkdir -p ${tmp_dir}
    mount -t devtmpfs none ${tmp_dir}
    mkdir -p ${tmp_dir}/pts
    mount --move /dev/pts ${tmp_dir}/pts
    umount /dev &>/dev/null || true
    mount --move ${tmp_dir} /dev
}

check_mount
devtmpfs_mount

err_file_path=${1}

# save current rights of error file
uid=$(stat -c '%u' ${err_file_path})
gid=$(stat -c '%g' ${err_file_path})

# give root permission
chown root:root ${err_file_path}

# redirect stderr to error file
exec 2>${err_file_path}

if [ ! -z ${DEBUG} ]; then
    echo "${@:2}"
fi

# now run the requested CMD
args="${@:2}"
bash -ec "${args}"
echo "return is $?"
