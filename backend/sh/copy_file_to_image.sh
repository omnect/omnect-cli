#!/bin/bash

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount
  losetup -D /tmp/image.wic
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -o output_file" 1>&2; exit 1;
}

while getopts ":i:o:" opt; do
    case "${opt}" in
        i)
            i=${OPTARG}
            ;;
        o)
            o=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

if [ -z "${i}" ] || [ -z "${o}" ]; then
    usage
fi

echo "i = ${i}"
echo "o = ${o}"

#filter partition from output file, valid values for part_pattern are "etc" or "data"
part_pattern=${o##/}
o=${part_pattern##*/}
part_pattern=${part_pattern%/*}
echo part_pattern=${part_pattern}

if [ ! "${part_pattern}" == "etc"  ] && [ ! "${part_pattern}" == "data"  ]; then
    echo error: output path doesnt begin with "/etc/" or "/data/" 1>&2 && exit 1
fi

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1

losetup -fP ${device} /tmp/image.wic
loopdev=$(losetup | grep /tmp/image.wic | awk '{print $1}')
echo loopdev=${loopdev}

for part in ${loopdev}p*
do
    if [ "${part_pattern}" == "$(e2label ${part} 2>/dev/null)" ]; then
        partloopdev=${part}
        break
    fi
done
echo partloopdev=${partloopdev}

[[ -z "${partloopdev}" ]] && echo "error: couldnt set up loopdev for input device image" 1>&2 && exit 1
mkdir /tmp/mount
mount -o loop,rw ${partloopdev} /tmp/mount
echo "cp ${i} /tmp/mount/upper/${o}"
cp ${i} /tmp/mount/upper/${o}
