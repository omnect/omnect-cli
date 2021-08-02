#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

d_echo ${0}

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
    set +o errexit
    umount /tmp/mount/${p}
    if [ ! -z "${g}" ] || [ ! -z "${u}" ]; then
        umount /tmp/mount/rootA
    fi
    losetup -d ${loopdev}
    while losetup ${loopdev} &>/dev/null; do sleep 0.1; done
    sync
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -d dirname -p partition -w wic_image [-g groupname] [-m chmod_mode ] [-u username]" 1>&2; exit 1;
}

while getopts "d:p:w:g:m:u:" opt; do
    case "${opt}" in
        d)
            d=${OPTARG}
            ;;
        g)
            g=${OPTARG}
            ;;
        m)
            m=${OPTARG}
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

if [ -z "${d}" ] || [ -z "${p}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "d = ${d}"
d_echo "g = ${g}"
d_echo "m = ${m}"
d_echo "p = ${p}"
d_echo "u = ${u}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && echo "error: input device image not found" 1>&2 && exit 1

# set up loop device to be able to mount image.wic
losetup_image_wic

# mount partition
part_pattern="${p}"
d_echo part_pattern="${p}"
mount_part

# mkdir
d_echo "mkdir -p /tmp/mount/${p}/${d}"
mkdir -p /tmp/mount/${p}/${d}

# mount rootA for permission handling
if [ ! -z "${g}" ] || [ ! -z "${u}" ]; then
    part_pattern="rootA"
    mount_part
fi

# group permission handling
if [ ! -z "${g}" ]; then
    gid=$(cat /tmp/mount/rootA/etc/group | grep ${g}: | awk 'BEGIN { FS = ":" } ; { print $3 }')
    d_echo $(ls -ld /tmp/mount/${p}/${d})
    d_echo "chgrp ${gid} /tmp/mount/${p}/${d}"
    chgrp ${gid} /tmp/mount/${p}/${d}
    d_echo $(ls -ld /tmp/mount/${p}/${d})
fi

# user permission handling
if [ ! -z "${u}" ]; then
    uid=$(cat /tmp/mount/rootA/etc/passwd | grep ${u}: | awk 'BEGIN { FS = ":" } ; { print $3 }')
    d_echo $(ls -ld /tmp/mount/${p}/${d})
    d_echo "chown ${uid} /tmp/mount/${p}/${d}"
    chown ${uid} /tmp/mount/${p}/${d}
    d_echo $(ls -ld /tmp/mount/${p}/${d})
fi

# chmod permission handling
if [ ! -z "${m}" ]; then
    d_echo $(ls -ld /tmp/mount/${p}/${d})
    d_echo "chmod ${m} /tmp/mount/${p}/${d}"
    chmod ${m} /tmp/mount/${p}/${d}
    d_echo $(ls -ld /tmp/mount/${p}/${d})
fi
