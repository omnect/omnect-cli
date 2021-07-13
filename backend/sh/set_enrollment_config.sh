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
    echo "Usage: $0 -e enrollment_input_file -p provisioning_input_file " 1>&2; exit 1;
}

while getopts ":e:p:" opt; do
    case "${opt}" in
        e)
            e=${OPTARG}
            ;;
        p)
            p=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

if [ -z "${e}" ] || [ -z "${p}" ]; then
    usage
fi

echo "e = ${e}"
echo "p = ${p}"

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${e} ]] && echo "error: enrollment input file \"${e}\" not found" 1>&2 && exit 1
[[ ! -f ${p} ]] && echo "error: provisioning input file \"${p}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount /tmp/image.wic
losetup_image_wic

# search and mount "etc" partion
part_pattern="etc"
mount_part

# copy enrollment_static.conf and provisioning_static.conf
mkdir -p /tmp/mount/etc/upper/ics_dm
echo "cp ${e} /tmp/mount/etc/upper/ics_dm/enrollment_static.conf"
cp ${e} /tmp/mount/etc/upper/ics_dm/enrollment_static.conf
echo "cp ${p} /tmp/mount/etc/upper/ics_dm/provisioning_static.conf"
cp ${p} /tmp/mount/etc/upper/ics_dm/provisioning_static.conf
