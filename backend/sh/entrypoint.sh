#!/bin/bash

# restore saved permissions for error file
function finish {
    chown $uid:$gid "$err_file_path"
}
trap finish EXIT

err_file_path="$1"

# save current rights of error file
uid=$(stat -c '%u' "$err_file_path")
gid=$(stat -c '%g' "$err_file_path")

# give root permission
chown root:root "$err_file_path"

# redirect stderr to error file
exec 2>$err_file_path

shift
# now run the requested CMD
"$@"
