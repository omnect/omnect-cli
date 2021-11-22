#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

d_echo ${0}

function usage() {
    echo "Usage: $0  -c identity_config -r root_cert [-d device_cert] [-k device_cert_key] -w wic_image" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "c:d:k:r:w:" opt; do
    case "${opt}" in
        c)
            c=${OPTARG}
            ;;
        d)
            d=${OPTARG}
            ;;
        k)
            k=${OPTARG}
            ;;
        r)
            r=${OPTARG}
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

# As long as x509 cert authentication isnt working for modul provisioning "-d"
# and "-k" are optional.
#
# if [ -z "${c}" ] || [ -z "${d}" ] || [ -z "${k}" ] || [ -z "${r}" ] || [ -z "${w}" ]; then
if [ -z "${c}" ] || [ -z "${r}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "c = ${c}"
d_echo "d = ${d}"
d_echo "k = ${k}"
d_echo "r = ${r}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"     && exit 1
#[[ ! -f ${d} ]] && error "input file \"${d}\" not found"   && exit 1
[[ ! -f ${c} ]] && error "input file \"${c}\" not found"    && exit 1
#[[ ! -f ${k} ]] && error "input file \"${k}\" not found"   && exit 1
[[ ! -f ${r} ]] && error "input file \"${r}\" not found"    && exit 1

uuid_gen

p=etc
read_in_partition

# copy identity config
d_echo e2cp ${c} /tmp/${uuid}/${p}.img:/upper/aziot/config.toml
e2mkdir /tmp/${uuid}/${p}.img:/upper/aziot
e2cp ${c} /tmp/${uuid}/${p}.img:/upper/aziot/config.toml

config_hostname ${c}
write_back_partition

# create/append to ics_dm_first_boot.sh in factory partition
# activate identity config on first boot
p=factory
read_in_partition
# for the following cp redirect stderr -> stdout, since it is possible that this file doesnt exist
e2cp /tmp/${uuid}/${p}.img:/ics_dm_first_boot.sh /tmp/${uuid}/icsd_dm_first_boot.sh 2>&1
echo "aziotctl config apply" >>  /tmp/${uuid}/ics_dm_first_boot.sh
e2cp /tmp/${uuid}/ics_dm_first_boot.sh /tmp/${uuid}/${p}.img:/ics_dm_first_boot.sh
write_back_partition

# copy root ca cert
# @todo refine how we use cert parition
p=data
read_in_partition
d_echo e2cp ${r} /tmp/${uuid}/${p}.img:/local/share/ca-certificates/$(basename ${r}).crt
e2mkdir /tmp/${uuid}/${p}.img:/local/share/ca-certificates
e2cp ${r} /tmp/${uuid}/${p}.img:/local/share/ca-certificates/$(basename ${r}).crt
write_back_partition
# Just a remark: copying the root ca cert isn't sufficient. The device has
# to call update-ca-certificates on first boot ... we handle that in
# ics-dm-first-boot.service
