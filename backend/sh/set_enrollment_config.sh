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
    echo "Usage: $0 -c enrollment_static.conf -w wic_image" 1>&2; exit 1;
}

while getopts ":c:w:" opt; do
    case "${opt}" in
        c)
            c=${OPTARG}
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

if [ -z "${c}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "c = ${c}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${c} ]] && echo "error: enrollment input file \"${c}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount image.wic
losetup_image_wic

# search and mount "etc" partion
part_pattern="etc"
mount_part

# copy enrollment_static.conf
mkdir -p /tmp/mount/etc/upper/ics_dm
d_echo "cp ${c} /tmp/mount/etc/upper/ics_dm/enrollment_static.conf"
cp ${c} /tmp/mount/etc/upper/ics_dm/enrollment_static.conf
