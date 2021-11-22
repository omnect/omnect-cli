#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

d_echo ${0}

function usage() {
    echo "Usage: $0  -c identity_config -w wic_image" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "c:w:" opt; do
    case "${opt}" in
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

d_echo "c = ${c}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"     && exit 1
[[ ! -f ${c} ]] && error "input file \"${c}\" not found"    && exit 1

uuid_gen

p=etc
read_in_partition

# copy identity config
d_echo "e2cp ${c} /tmp/${uuid}/${p}.img:/upper/aziot/config.toml"
e2mkdir /tmp/${uuid}/${p}.img:/upper/aziot/
e2cp ${c} /tmp/${uuid}/${p}.img:/upper/aziot/config.toml

config_hostname ${c}
write_back_partition

# create/append to ics_dm_first_boot.sh in factory partition
# activate identity config on first boot depending on device variant (edge / non edge)
p=factory
read_in_partition
# for the following cp redirect stderr -> stdout, since it is possible that this file doesnt exist
e2cp /tmp/${uuid}/${p}.img:/ics_dm_first_boot.sh /tmp/${uuid}/icsd_dm_first_boot.sh 2>&1
e2cp /tmp/${uuid}/rootA.img:/usr/lib/os-release /tmp/${uuid}/os-release
if [ $(cat /tmp/${uuid}/os-release | grep ^DISTRO_FEATURES | grep ' iotedge ' | wc -l) -eq 1 ]; then
    echo "iotedge config apply" >> /tmp/${uuid}/ics_dm_first_boot.sh
else
    echo "aziotctl config apply" >> /tmp/${uuid}/ics_dm_first_boot.sh
fi
e2cp /tmp/${uuid}/ics_dm_first_boot.sh /tmp/${uuid}/${p}.img:/ics_dm_first_boot.sh
write_back_partition
