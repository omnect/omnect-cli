# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount
  losetup -D /tmp/image.wic
}
trap finish EXIT

function usage() {
    echo "Usage: $0 -d device_cert -i identity_config -k device_cert_key -r root_cert" 1>&2; exit 1;
}

function search_part_loopdev() {
    for part in ${loopdev}p*
    do
        if [ "${part_pattern}" == "$(e2label ${part} 2>/dev/null)" ]; then
            partloopdev=${part}
            break
        fi
    done
    echo partloopdev=${partloopdev}
}

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

while getopts ":d:i:k:r:" opt; do
    case "${opt}" in
        d)
            d=${OPTARG}
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

if [ -z "${d}" ] || [ -z "${i}" ] || [ -z "${k}" ] || [ -z "${r}" ]; then
    usage
fi

echo "d = ${d}"
echo "i = ${i}"
echo "k = ${k}"
echo "r = ${r}"

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${d} ]] && echo "error: input file \"${d}\" not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1
[[ ! -f ${r} ]] && echo "error: input file \"${r}\" not found" 1>&2 && exit 1
[[ ! -f ${s} ]] && echo "error: input file \"${s}\" not found" 1>&2 && exit 1

# todo verify input identity config for "hostname", "trust_bundle_cert", "edge_ca" sections
# this script enforces a default placement of certs, e.g.
# [trust_bundle_cert]
# # root ca:
# trust_bundle_cert = "file:///etc/aziot/trust-bundle.pem"
# [edge_ca]
# # device cert + key:
# cert = "file:///etc/aziot/edge-ca.pem"
# pk = "file:///etc/aziot/edge-ca.key.pem"

# set up loop device to be able to mount /tmp/image.wic
losetup -fP ${device} /tmp/image.wic
loopdev=$(losetup | grep /tmp/image.wic | awk '{print $1}')
echo loopdev=${loopdev}

# search and mount "rootA" partion
part_pattern="rootA"
search_part_loopdev

[[ -z "${partloopdev}" ]] && echo "error: couldnt set up loopdev for input device image (part_pattern: ${part_pattern})" 1>&2 && exit 1
mkdir -p /tmp/mount
mount -o loop ${partloopdev} /tmp/mount

# copy identity config
aziot_gid=$(cat /tmp/mount/etc/group | grep aziot: | awk 'BEGIN { FS = ":" } ; { print $3 }')
echo cp ${i} /tmp/mount/etc/aziot/config.toml
chgrp ${aziot_gid} /tmp/mount/etc/aziot/config.toml
cp ${i} /tmp/mount/etc/aziot/config.toml
chmod a+r /tmp/mount/etc/aziot/config.toml

# copy root ca cert
echo cp ${r} /tmp/mount/etc/aziot/trust-bundle.pem
cp ${r} /tmp/mount/etc/aziot/trust-bundle.pem
chmod a+r /tmp/mount/etc/aziot/trust-bundle.pem

# copy device cert and key
echo cp ${d} /tmp/mount/etc/aziot/edge-ca.pem
cp ${d} /tmp/mount/etc/aziot/edge-ca.pem
chmod a+r /tmp/mount/etc/aziot/edge-ca.pem
echo cp ${k} /tmp/mount/etc/aziot/edge-ca.key.pem
cp ${k} /tmp/mount/etc/aziot/edge-ca.key.pem
chmod a+r /tmp/mount/etc/aziot/edge-ca.key.pem
