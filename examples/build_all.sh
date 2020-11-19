#!/bin/bash
set -e
pushd $(dirname ${BASH_SOURCE[0]})

for d in "status-message"  $(ls -d */ | grep -v -e "status-message\/$"); do
    echo building $d;
    (cd "$d"; ./build.sh);
done

popd