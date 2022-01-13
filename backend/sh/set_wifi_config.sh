#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

d_echo ${0}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -w wic_image [-b output_bmap_file]" 1>&2; exit 1;
}

while getopts "b:i:w:" opt; do
    case "${opt}" in
        b)
            b=${OPTARG}
            ;;
        i)
            i=${OPTARG}
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

if [ -z "${i}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "b = ${b}"
d_echo "i = ${i}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"   && exit 1
[[ ! -f ${i} ]] && error "input file \"${i}\" not found"  && exit 1

uuid_gen

p=factory
read_in_partition

# copy wpa_supplicant conf
d_echo "e2cp ${i} /tmp/${uuid}/${p}.img:/etc/wpa_supplicant/wpa_supplicant-wlan0.conf"
e2mkdir /tmp/${uuid}/${p}.img:/etc/wpa_supplicant
e2cp -P 644 ${i} /tmp/${uuid}/${p}.img:/etc/wpa_supplicant/wpa_supplicant-wlan0.conf

write_back_partition

if [ "0" != "${b}0" ]; then
    bmaptool create -o ${b} ${w}
fi
