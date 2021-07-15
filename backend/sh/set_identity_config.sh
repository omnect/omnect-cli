#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount/data
  umount /tmp/mount/etc
  umount /tmp/mount/rootA
  losetup -D /tmp/image.wic
}
trap finish EXIT

function usage() {
    echo "Usage: $0  -i identity_config" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts ":d:i:k:r:" opt; do
    case "${opt}" in
        i)
            i=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

if [ -z "${i}" ]; then
    usage
fi

echo "i = ${i}"

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount /tmp/image.wic
losetup_image_wic

# search and mount "etc" partion
part_pattern="etc"
mount_part

# search and mount "data" partion
part_pattern="data"
mount_part

# search and mount "rootA" partion
part_pattern="rootA"
mount_part

# copy identity config
aziot_gid=$(cat /tmp/mount/rootA/etc/group | grep aziot: | awk 'BEGIN { FS = ":" } ; { print $3 }')
mkdir -p /tmp/mount/etc/upper/aziot/
echo cp ${i} /tmp/mount/etc/upper/aziot/config.toml
cp ${i} /tmp/mount/etc/upper/aziot/config.toml
chgrp ${aziot_gid} /tmp/mount/etc/upper/aziot/config.toml
chmod a+r,g+w /tmp/mount/etc/upper/aziot/config.toml

# activate identity config on first boot
# here it is okay to alter a file in the root partition
echo "aziotctl config apply" >> /tmp/mount/rootA/usr/bin/ics_dm_first_boot.sh

# set hostname
hostname=$(grep "^hostname" ${i} | cut -d "=" -f2 | xargs)
echo "set hostname to ${hostname}"
echo "${hostname}" > /tmp/mount/etc/upper/hostname
cp /tmp/mount/rootA/etc/hosts /tmp/mount/etc/upper/
sed -i "s/^127.0.1.1\(.*\)/127.0.1.1 ${hostname}/" /tmp/mount/etc/upper/hosts
