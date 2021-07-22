#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount/etc
  losetup -d ${loopdev}
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -w wic_image" 1>&2; exit 1;
}

while getopts ":i:w:" opt; do
    case "${opt}" in
        i)
            i=${OPTARG}
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

if [ -z "${i}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "i = ${i}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount image.wic
losetup_image_wic

# search and mount "etc" partion
part_pattern="etc"
mount_part

# copy wpa_supplicant conf
mkdir -p /tmp/mount/etc/upper/wpa_supplicant
d_echo "cp ${i} /tmp/mount/etc/upper/wpa_supplicant/wpa_supplicant-wlan0.conf"
cp ${i} /tmp/mount/etc/upper/wpa_supplicant/wpa_supplicant-wlan0.conf

# enable wpa_supplicant service
mkdir -p /tmp/mount/etc/upper/systemd/system/multi-user.target.wants
ln -sf /lib/systemd/system/wpa_supplicant@.service /tmp/mount/etc/upper/systemd/system/multi-user.target.wants/wpa_supplicant@wlan0.service
