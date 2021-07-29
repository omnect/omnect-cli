#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
    set +o errexit
    umount /tmp/mount/${p}
    if [ ! -z "${g}" ] || [ ! -z "${u}" ]; then
        umount /tmp/mount/rootA
    fi
    losetup -d ${loopdev}
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -o output_file -p partition -w wic_image [-g groupname] [-m chmod_mode ] [-u username]" 1>&2; exit 1;
}

while getopts "i:o:p:w:g:m:u:" opt; do
    case "${opt}" in
        i)
            i=${OPTARG}
            ;;
        g)
            g=${OPTARG}
            ;;
        m)
            m=${OPTARG}
            ;;
        o)
            o=${OPTARG}
            ;;
        p)
            p=${OPTARG}
            ;;
        u)
            u=${OPTARG}
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

if [ -z "${i}" ] || [ -z "${o}" ] || [ -z "${p}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "i = ${i}"
d_echo "g = ${g}"
d_echo "m = ${m}"
d_echo "o = ${o}"
d_echo "p = ${p}"
d_echo "u = ${u}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount image.wic
losetup_image_wic

# mount partition
part_pattern="${p}"
d_echo part_pattern="${p}"
mount_part

# copy file
mkdir -p $(dirname /tmp/mount/${p}/${o})
d_echo "cp ${i} /tmp/mount/${p}/${o}"
cp ${i} /tmp/mount/${p}/${o}

# mount rootA for permission handling
if [ ! -z "${g}" ] || [ ! -z "${u}" ]; then
    part_pattern="rootA"
    mount_part
fi

# group permission handling
if [ ! -z "${g}" ]; then
    gid=$(cat /tmp/mount/rootA/etc/group | grep ${g}: | awk 'BEGIN { FS = ":" } ; { print $3 }')
    d_echo $(ls -l /tmp/mount/${p}/${o})
    d_echo "chgrp ${gid} /tmp/mount/${p}/${o}"
    chgrp ${gid} /tmp/mount/${p}/${o}
    d_echo $(ls -l /tmp/mount/${p}/${o})
fi

# user permission handling
if [ ! -z "${u}" ]; then
    uid=$(cat /tmp/mount/rootA/etc/passwd | grep ${u}: | awk 'BEGIN { FS = ":" } ; { print $3 }')
    d_echo $(ls -l /tmp/mount/${p}/${o})
    d_echo "chown ${uid} /tmp/mount/${p}/${o}"
    chown ${uid} /tmp/mount/${p}/${o}
    d_echo $(ls -l /tmp/mount/${p}/${o})
fi

# chmod permission handling
if [ ! -z "${m}" ]; then
    d_echo $(ls -l /tmp/mount/${p}/${o})
    d_echo "chmod ${m} /tmp/mount/${p}/${o}"
    chmod ${m} /tmp/mount/${p}/${o}
    d_echo $(ls -l /tmp/mount/${p}/${o})
fi
