#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount
  losetup -d ${loopdev}
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -o output_file -w wic_image" 1>&2; exit 1;
}

while getopts ":i:o:w:" opt; do
    case "${opt}" in
        i)
            i=${OPTARG}
            ;;
        o)
            o=${OPTARG}
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

if [ -z "${i}" ] || [ -z "${o}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "i = ${i}"
d_echo "o = ${o}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount image.wic
losetup_image_wic

#filter partition from output file, valid values for part_pattern are "etc" or "data"
part_pattern=${o##/}
o=${part_pattern##*/}
part_pattern=${part_pattern%/*}
d_echo part_pattern=${part_pattern}

if [ ! "${part_pattern}" == "etc"  ] && [ ! "${part_pattern}" == "data"  ]; then
    echo error: output path doesnt begin with "/etc/" or "/data/" 1>&2 && exit 1
fi
# finally mount it
mount_part

mkdir /tmp/mount
mount -o loop,rw ${partloopdev} /tmp/mount
mkdir -p /tmp/mount/upper
d_echo "cp ${i} /tmp/mount/upper/${o}"
cp ${i} /tmp/mount/upper/${o}
