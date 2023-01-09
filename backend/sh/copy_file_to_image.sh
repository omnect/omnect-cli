#!/bin/bash

# include shared functions
. /omnect-sh/functions

d_echo ${0}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -o output_file -p partition -w wic_image [-b output_bmap_file]" 1>&2; exit 1;
}

while getopts "b:i:o:p:w:" opt; do
    case "${opt}" in
        b)
            b=${OPTARG}
            ;;
        i)
            i=${OPTARG}
            ;;
        o)
            o=${OPTARG}
            ;;
        p)
            p=${OPTARG}
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

d_echo "b = ${b}"
d_echo "i = ${i}"
d_echo "o = ${o}"
d_echo "p = ${p}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"     && exit 1
[[ ! -f ${i} ]] && error "input file \"${i}\" not found"    && exit 1

uuid_gen
handle_partition_type
read_in_partition

# copy file
if [ "${p}" != "boot" ]; then
    d_echo "e2cp ${i} /tmp/${uuid}/${p}.img:${o}"
    e2mkdir /tmp/${uuid}/${p}.img:$(dirname ${o})
    e2cp ${i} /tmp/${uuid}/${p}.img:${o}
else
    if [ ! -d $(dirname ${o}) ]; then
        d_echo "mmd -i /tmp/${uuid}/${p}.img ::$(dirname ${o})"
        mmd -i /tmp/${uuid}/${p}.img ::$(dirname ${o})
    fi

    d_echo "mcopy -o -i /tmp/${uuid}/${p}.img ${i} ::${o})"
    mcopy -o -i /tmp/${uuid}/${p}.img ${i} ::${o}
fi

write_back_partition

if [ "0" != "${b}0" ]; then
    bmaptool create -o ${b} ${w}
fi
