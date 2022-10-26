#!/bin/bash

# include shared functions
. /omnect-sh/functions

d_echo ${0}

function usage() {
    echo "Usage: $0 -c identity_config -e edge_device_cert -k edge_device_cert_key -r root_cert -w wic_image [-b output_bmap_file]" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "b:c:e:k:r:w:" opt; do
    case "${opt}" in
        b)
            b=${OPTARG}
            ;;
        c)
            c=${OPTARG}
            ;;
        e)
            e=${OPTARG}
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

if [ -z "${c}" ] || [ -z "${e}" ] || [ -z "${k}" ] || [ -z "${r}" ] || [ -z "${w}" ]; then
    usage
fi

d_echo "b = ${b}"
d_echo "c = ${c}"
d_echo "e = ${e}"
d_echo "k = ${k}"
d_echo "r = ${r}"
d_echo "w = ${w}"

[[ ! -f ${w} ]] && error "input device image not found"   && exit 1
[[ ! -f ${c} ]] && error "input file \"${c}\" not found"  && exit 1
[[ ! -f ${e} ]] && error "input file \"${e}\" not found"  && exit 1
[[ ! -f ${k} ]] && error "input file \"${k}\" not found"  && exit 1
[[ ! -f ${r} ]] && error "input file \"${r}\" not found"  && exit 1

# this script enforces a default placement of certs, e.g.
# [trust_bundle_cert]
# # root ca:
# trust_bundle_cert = "file:///var/secrets/trust-bundle.pem"
# [edge_ca]
# # device cert + key:
# cert = "file:///var/secrets/edge-ca.pem"
# pk = "file:///var/secrets/edge-ca.key.pem"

uuid_gen

p=factory
read_in_partition

copy_identity_config

write_back_partition

# copy root ca cert,  device cert and key
p=cert
read_in_partition

e2mkdir /tmp/${uuid}/${p}.img:/ca
e2mkdir /tmp/${uuid}/${p}.img:/priv
d_echo e2cp -P 644 ${r} /tmp/${uuid}/${p}.img:/ca/trust-bundle.pem.crt
e2cp -P 644 ${r} /tmp/${uuid}/${p}.img:/ca/trust-bundle.pem.crt
d_echo e2cp -P 644 ${e} /tmp/${uuid}/${p}.img:/priv/edge-ca.pem
e2cp -P 644 ${e} /tmp/${uuid}/${p}.img:/priv/edge-ca.pem
d_echo e2cp -P 644 ${k} /tmp/${uuid}/${p}.img:/priv/edge-ca.key.pem
e2cp -P 644  ${k} /tmp/${uuid}/${p}.img:/priv/edge-ca.key.pem

write_back_partition

if [ "0" != "${b}0" ]; then
    bmaptool create -o ${b} ${w}
fi
