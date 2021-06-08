# exit handler which makes sure we dont leave an undefined host state regarding loop devices
function finish {
  set +o errexit
  umount /tmp/mount
  losetup -D /tmp/image.wic
}
trap finish EXIT

set -o errexit   # abort on nonzero exitstatus
set -o pipefail  # don't hide errors within pipes

function usage() {
    echo "Usage: $0 -d device_cert -i identity_config -k device_cert_key -r root_cert -s service_cert " 1>&2; exit 1;
}

while getopts ":d:i:k:r:s:" opt; do
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
        s)
            s=${OPTARG}
            ;;
        *)
            usage
            ;;
    esac
done
shift $((OPTIND-1))

if [ -z "${d}" ] || [ -z "${i}" ] || [ -z "${k}" ] || [ -z "${r}" ] || [ -z "${s}" ]; then
    usage
fi

echo "d = ${d}"
echo "i = ${i}"
echo "r = ${r}"
echo "s = ${s}"

[[ ! -f /tmp/image.wic ]] && echo "error: input device image not found" 1>&2 && exit 1
[[ ! -f ${d} ]] && echo "error: input file \"${d}\" not found" 1>&2 && exit 1
[[ ! -f ${i} ]] && echo "error: input file \"${i}\" not found" 1>&2 && exit 1
[[ ! -f ${r} ]] && echo "error: input file \"${r}\" not found" 1>&2 && exit 1
[[ ! -f ${s} ]] && echo "error: input file \"${s}\" not found" 1>&2 && exit 1

# set up loop device to be able to mount /tmp/image.wic
losetup -fP ${device} /tmp/image.wic
loopdev=$(losetup | grep /tmp/image.wic | awk '{print $1}')
echo loopdev=${loopdev}

# search and mount "etc" partition
part_pattern="etc"
for part in ${loopdev}p*
do
    if [ "${part_pattern}" == "$(e2label ${part} 2>/dev/null)" ]; then
        partloopdev=${part}
        break
    fi
done
echo partloopdev=${partloopdev}

[[ -z "${partloopdev}" ]] && echo "error: couldnt set up loopdev for input device image (part_pattern: ${part_pattern})" 1>&2 && exit 1
mkdir -p /tmp/mount/etc
mount -o loop,rw ${partloopdev} /tmp/mount/etc

# search and mount "rootA" partion
part_pattern="rootA"
for part in ${loopdev}p*
do
    if [ "${part_pattern}" == "$(e2label ${part} 2>/dev/null)" ]; then
        partloopdev=${part}
        break
    fi
done
echo partloopdev=${partloopdev}

[[ -z "${partloopdev}" ]] && echo "error: couldnt set up loopdev for input device image (part_pattern: ${part_pattern})" 1>&2 && exit 1
mkdir -p /tmp/mount/rootA
mount -o loop,ro ${partloopdev} /tmp/mount/rootA

# copy identity config
mkdir -p /tmp/mount/upper/aziot
aziot_gid=$(cat /tmp/mount/rootA/etc/group | grep aziot: | awk 'BEGIN { FS = ":" } ; { print $3 }')
chgrp ${aziot_gid} /tmp/mount/upper/aziot
cp ${i} /tmp/mount/upper/aziot/config.toml
chmod a+r /tmp/mount/upper/aziot/config.toml

# copy root cert
