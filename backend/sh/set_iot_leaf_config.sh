#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
    set +o errexit
    umount /tmp/mount/data
    umount /tmp/mount/etc
    umount /tmp/mount/rootA
    losetup -d ${loopdev}
}
trap finish EXIT

function usage() {
    echo "Usage: $0  -c identity_config -r root_cert [-d device_cert] [-k device_cert_key] -w wic_image" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts ":c:d:k:r:w:" opt; do
    case "${opt}" in
        c)
            c=${OPTARG}
            ;;
        d)
            d=${OPTARG}
            ;;
        k)
            k=${OPTARG}
            ;;
        r)
            r=${OPTARG}
            ;;
        w)
            w=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

# As long as x509 cert authentication isnt working for modul provisioning "-d"
# and "-k" are optional.
#
# if [ -z "${c}" ] || [ -z "${d}" ] || [ -z "${k}" ] || [ -z "${r}" ] || [ -z "${w}" ]; then
if [ -z "${c}" ] || [ -z "${r}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "c = ${c}"
d_echo "d = ${d}"
d_echo "k = ${k}"
d_echo "r = ${r}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && echo "error: input device image not found" 1>&2 && exit 1
#[[ ! -f ${d} ]] && echo "error: input file \"${d}\" not found" 1>&2 && exit 1
[[ ! -f ${c} ]] && echo "error: input file \"${c}\" not found" 1>&2 && exit 1
#[[ ! -f ${k} ]] && echo "error: input file \"${k}\" not found" 1>&2 && exit 1
[[ ! -f ${r} ]] && echo "error: input file \"${r}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount image.wic
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
d_echo cp ${c} /tmp/mount/etc/upper/aziot/config.toml
cp ${c} /tmp/mount/etc/upper/aziot/config.toml
chgrp ${aziot_gid} /tmp/mount/etc/upper/aziot/config.toml
chmod a+r,g+w /tmp/mount/etc/upper/aziot/config.toml

# activate identity config on first boot
# here it is okay to alter a file in the root partition
echo "aziotctl config apply" >> /tmp/mount/rootA/usr/bin/ics_dm_first_boot.sh

# set hostname
hostname=$(grep "^hostname" ${c} | cut -d "=" -f2 | xargs)
d_echo "set hostname to ${hostname}"
echo "${hostname}" > /tmp/mount/etc/upper/hostname
cp /tmp/mount/rootA/etc/hosts /tmp/mount/etc/upper/
sed -i "s/^127.0.1.1\(.*\)/127.0.1.1 ${hostname}/" /tmp/mount/etc/upper/hosts

# copy root ca cert
mkdir -p /tmp/mount/data/local/share/ca-certificates/
d_echo cp ${r} /tmp/mount/data/local/share/ca-certificates/$(basename ${r}).crt
cp ${r} /tmp/mount/data/local/share/ca-certificates/$(basename ${r}).crt
# Just a remark: copying the root ca cert isn't sufficient. The device has
# to call update-ca-certificates on first boot ... we handle that in
# ics-dm-first-boot.service
