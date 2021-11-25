#!/bin/bash

# include shared functions
. /ics-dm-sh/functions

# restore saved permissions for error file
function finish {
    chown ${uid}:${gid} ${err_file_path}
    echo "entrypoint return=${return}"
    sync
}
trap finish EXIT

err_file_path=${1}

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
