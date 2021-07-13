#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount/data
  umount /tmp/mount/etc
  umount /tmp/mount/rootA
  losetup -D /tmp/image.wic
}
trap finish EXIT

function usage() {
    echo "Usage: $0 -e edge_device_cert -i identity_config -k edge_device_cert_key -r root_cert" 1>&2; exit 1;
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts ":e:i:k:r:" opt; do
    case "${opt}" in
        e)
            e=${OPTARG}
            ;;
        i)
            i=${OPTARG}
            ;;
        k)
            k=${OPTARG}
            ;;
        r)
            r=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

if [ -z "${e}" ] || [ -z "${i}" ] || [ -z "${k}" ] || [ -z "${r}" ]; then
    usage
fi

echo "e = ${e}"
echo "i = ${i}"
echo "k = ${k}"
echo "r = ${r}"

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${e} ]] && echo "error: input file \"${e}\" not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1
[[ ! -f ${k} ]] && echo "error: input file \"${k}\" not found" 1>&2 && exit 1
[[ ! -f ${r} ]] && echo "error: input file \"${r}\" not found" 1>&2 && exit 1

# this script enforces a default placement of certs, e.g.
# [trust_bundle_cert]
# # root ca:
# trust_bundle_cert = "file:///var/secrets/trust-bundle.pem"
# [edge_ca]
# # device cert + key:
# cert = "file:///var/secrets/edge-ca.pem"
# pk = "file:///var/secrets/edge-ca.key.pem"

# set up loop device to be able to mount /tmp/image.wic
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
mkdir -p /tmp/mount/etc/upper/aziot/
echo cp ${i} /tmp/mount/etc/upper/aziot/config.toml
cp ${i} /tmp/mount/etc/upper/aziot/config.toml
chgrp ${aziot_gid} /tmp/mount/etc/upper/aziot/config.toml
chmod a+r,g+w /tmp/mount/etc/upper/aziot/config.toml

# activate identity config on first boot if enrollment demo is not installed
# here it is okay to alter a file in the root partition
if [ ! -e /tmp/mount/rootA/etc/systemd/system/multi-user.target.wants/enrollment.service ]; then
    echo "iotedge config apply" >> /tmp/mount/rootA/usr/bin/ics_dm_first_boot.sh
fi

# set hostname
hostname=$(grep "^hostname" ${i} | cut -d "=" -f2 | xargs)
echo "set hostname to ${hostname}"
echo "${hostname}" > /tmp/mount/etc/upper/hostname
cp /tmp/mount/rootA/etc/hosts /tmp/mount/etc/upper/
sed -i "s/^127.0.1.1\(.*\)/127.0.1.1 ${hostname}/" /tmp/mount/etc/upper/hosts

# copy root ca cert
mkdir -p /tmp/mount/data/var/secrets
echo cp ${r} /tmp/mount/data/var/secrets/trust-bundle.pem
cp ${r} /tmp/mount/data/var/secrets/trust-bundle.pem
chmod a+r /tmp/mount/data/var/secrets/trust-bundle.pem

# copy device cert and key
echo cp ${e} /tmp/mount/data/var/secrets/edge-ca.pem
cp ${e} /tmp/mount/data/var/secrets/edge-ca.pem
chmod a+r /tmp/mount/data/var/secrets/edge-ca.pem
echo cp ${k} /tmp/mount/data/var/secrets/edge-ca.key.pem
cp ${k} /tmp/mount/data/var/secrets/edge-ca.key.pem
chmod a+r /tmp/mount/data/var/secrets/edge-ca.key.pem
