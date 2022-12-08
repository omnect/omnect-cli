#!/bin/bash

# include shared functions
. /omnect-sh/functions

d_echo ${0}

function usage() {
    echo "Usage: $0  -c identity_config -w wic_image [-b output_bmap_file]" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "b:c:w:" opt; do
    case "${opt}" in
        b)
            b=${OPTARG}
            ;;
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

d_echo "b = ${b}"
d_echo "c = ${c}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"     && exit 1
[[ ! -f ${c} ]] && error "input file \"${c}\" not found"    && exit 1

uuid_gen

handle_partition_type
p=factory
read_in_partition

# copy identity config
copy_identity_config

write_back_partition

if [ "0" != "${b}0" ]; then
    bmaptool create -o ${b} ${w}
fi
