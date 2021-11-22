#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

d_echo ${0}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -i input_file -w wic_image" 1>&2; exit 1;
}

while getopts "i:w:" opt; do
    case "${opt}" in
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

d_echo "i = ${i}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"   && exit 1
[[ ! -f ${i} ]] && error "input file \"${i}\" not found"  && exit 1

uuid_gen

p=etc
read_in_partition

# copy wpa_supplicant conf
d_echo "e2cp ${i} /tmp/${uuid}/${p}.img:/upper/wpa_supplicant/wpa_supplicant-wlan0.conf"
e2mkdir /tmp/${uuid}/${p}.img:/upper/wpa_supplicant
e2cp -P 644 ${i} /tmp/${uuid}/${p}.img:/upper/wpa_supplicant/wpa_supplicant-wlan0.conf

write_back_partition

# enable wpa_supplicant
# create/append to ics_dm_first_boot.sh in factory partition
p=factory
read_in_partition
# for the following cp redirect stderr -> stdout, since it is possible that this file doesnt exist
e2cp /tmp/${uuid}/${p}.img:/ics_dm_first_boot.sh /tmp/${uuid}/icsd_dm_first_boot.sh 2>&1
echo "systemctl enable wpa_supplicant@wlan0.service && systemctl start wpa_supplicant@wlan0.service" >> /tmp/${uuid}/ics_dm_first_boot.sh
e2cp /tmp/${uuid}/ics_dm_first_boot.sh /tmp/${uuid}/${p}.img:/ics_dm_first_boot.sh
write_back_partition
