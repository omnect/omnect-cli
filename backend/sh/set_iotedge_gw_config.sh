#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

d_echo ${0}

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
    set +o errexit
    umount /tmp/mount/data
    umount /tmp/mount/etc
    umount /tmp/mount/rootA
    detach_loopdev
}
trap finish EXIT

function usage() {
    echo "Usage: $0 -c identity_config -e edge_device_cert -k edge_device_cert_key -r root_cert -w wic_image" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts "c:e:k:r:w:" opt; do
    case "${opt}" in
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

# set up loop device to be able to mount image.wic
losetup_image_wic

# search and mount "etc" partion
part_pattern="etc"
mount_part

# search and mount "data" partion
part_pattern="data"
mount_part

# search and mount "rootA" partion
part_pattern="rootA"
mount_part

# copy identity config
aziot_gid=$(cat /tmp/mount/rootA/etc/group | grep aziot: | awk 'BEGIN { FS = ":" } ; { print $3 }')
mkdir -p /tmp/mount/etc/upper/aziot
chmod 0770 /tmp/mount/etc/upper/aziot
chgrp ${aziot_gid} /tmp/mount/etc/upper/aziot
d_echo cp ${c} /tmp/mount/etc/upper/aziot/config.toml
cp ${c} /tmp/mount/etc/upper/aziot/config.toml
chgrp ${aziot_gid} /tmp/mount/etc/upper/aziot/config.toml
chmod a+r,g+w /tmp/mount/etc/upper/aziot/config.toml

# activate identity config on first boot if enrollment demo is not installed
# here it is okay to alter a file in the root partition
if [ ! -e /tmp/mount/rootA/etc/systemd/system/multi-user.target.wants/enrollment.service ]; then
    echo "iotedge config apply" >> /tmp/mount/rootA/usr/bin/ics_dm_first_boot.sh
fi

# config hostname
config_hostname ${c}

# copy root ca cert
mkdir -p /tmp/mount/data/var/secrets
d_echo cp ${r} /tmp/mount/data/var/secrets/trust-bundle.pem
cp ${r} /tmp/mount/data/var/secrets/trust-bundle.pem
chmod a+r /tmp/mount/data/var/secrets/trust-bundle.pem

# copy device cert and key
d_echo cp ${e} /tmp/mount/data/var/secrets/edge-ca.pem
cp ${e} /tmp/mount/data/var/secrets/edge-ca.pem
chmod a+r /tmp/mount/data/var/secrets/edge-ca.pem
d_echo cp ${k} /tmp/mount/data/var/secrets/edge-ca.key.pem
cp ${k} /tmp/mount/data/var/secrets/edge-ca.key.pem
chmod a+r /tmp/mount/data/var/secrets/edge-ca.key.pem
