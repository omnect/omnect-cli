#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount/etc
  losetup -D /tmp/image.wic
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -c enrollment_input_file " 1>&2; exit 1;
}

while getopts ":c:" opt; do
    case "${opt}" in
        c)
            c=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

if [ -z "${c}" ]; then
    usage
fi

echo "c = ${c}"

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${c} ]] && echo "error: enrollment input file \"${c}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount /tmp/image.wic
losetup_image_wic

# search and mount "etc" partion
part_pattern="etc"
mount_part

# copy enrollment_static.conf and provisioning_static.conf
mkdir -p /tmp/mount/etc/upper/ics_dm
echo "cp ${c} /tmp/mount/etc/upper/ics_dm/enrollment_static.conf"
cp ${c} /tmp/mount/etc/upper/ics_dm/enrollment_static.conf
