#!/bin/bash

# include shared functions
. /omnect-sh/functions

d_echo ${0}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -o output_file -p partition -w wic_image" 1>&2; exit 1;
}

while getopts "b:i:o:p:w:" opt; do
    case "${opt}" in
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

d_echo "i = ${i}"
d_echo "o = ${o}"
d_echo "p = ${p}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"     && exit 1
[[ ! -f ${o} ]] && error "output file file \"${i}\" not found"    && exit 1

uuid_gen
handle_partition_type
read_in_partition

# copy file
if [ "${p}" != "boot" ]; then
    d_echo "e2cp /tmp/${uuid}/${p}.img:${i}" ${o}
    e2cp /tmp/${uuid}/${p}.img:${i} ${o}
else
    out_dir=$(dirname ${o})
    d_echo "mcopy -o -i /tmp/"${uuid}"/"${p}".img ::"${i}" "${out_dir}""
    mcopy -o -i /tmp/"${uuid}"/"${p}".img ::"${i}" "${out_dir}" || (error "mcopy "${o}" failed" && exit 1)
    
    d_echo "cat "${out_dir}"/$(basename "${i}") > "${o}""
    cat "${out_dir}"/$(basename "${i}") > "${o}"
fi
