#!/bin/bash

# include shared functions
. /omnect-sh/functions

d_echo ${0}

function usage() {
    echo "Usage: $0  -r root_cert -d device_id -w wic_image [-b output_bmap_file]" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "r:d:w:" opt; do
    case "${opt}" in
        r)
            r=${OPTARG}
            ;;
        d)
            d=${OPTARG}
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

if [ -z "${r}" ] || [ -z "${d}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "r = ${r}"
d_echo "d = ${d}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"     && exit 1

p=cert

uuid_gen
handle_partition_type
read_in_partition

echo "${d}" > /tmp/${uuid}/authorized_principals

e2cp "${r}" /tmp/${uuid}/${p}.img:/ssh/root_ca
e2cp /tmp/${uuid}/authorized_principals /tmp/${uuid}/${p}.img:/ssh/authorized_principals

write_back_partition

if [ "0" != "${b}0" ]; then
    bmaptool create -o ${b} ${w}
fi
