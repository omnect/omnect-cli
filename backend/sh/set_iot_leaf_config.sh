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
[[ ! -f ${k} ]] && echo "error: input file \"${k}\" not found" 1>&2 && exit 1
[[ ! -f ${r} ]] && echo "error: input file \"${r}\" not found" 1>&2 && exit 1
