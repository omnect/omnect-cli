#!/bin/bash

# include shared functions
. /omnect-sh/functions

# restore saved permissions for error file
function finish {
    chown ${uid}:${gid} ${err_file_path}
    echo "entrypoint return=${return}"
    errors=$(cat ${err_file_path})
    [ -n "${errors}" ] && echo "errors:\n ${errors}"
    sync
}
trap finish EXIT

err_file_path=${1}
echo "err_file_path=${err_file_path}"

# save current rights of error file
uid=$(stat -c '%u' ${err_file_path})
gid=$(stat -c '%g' ${err_file_path})

# give root permission
chown root:root ${err_file_path}

# redirect stderr to error file
exec 2>${err_file_path}

d_echo "${@:2}"

# now run the requested CMD
args="${@:2}"
bash -ec "${args}"
return=${?}
