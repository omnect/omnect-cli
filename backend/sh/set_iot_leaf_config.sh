#!/bin/bash

# include shared functions
. /omnect-sh/functions

d_echo ${0}

function usage() {
    echo "Usage: $0  -c identity_config -r root_cert [-d device_cert] [-k device_cert_key] -w wic_image [-b output_bmap_file]" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "b:c:d:k:r:w:" opt; do
    case "${opt}" in
        b)
            b=${OPTARG}
            ;;
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

d_echo "b = ${b}"
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

p=factory
read_in_partition

copy_identity_config

write_back_partition

# copy root ca cert
p=cert
read_in_partition

d_echo e2cp ${r} /tmp/${uuid}/${p}.img:/ca/$(basename ${r}).crt
e2mkdir /tmp/${uuid}/${p}.img:/ca
e2cp ${r} /tmp/${uuid}/${p}.img:/ca/$(basename ${r}).crt

write_back_partition

if [ "0" != "${b}0" ]; then
    bmaptool create -o ${b} ${w}
fi
