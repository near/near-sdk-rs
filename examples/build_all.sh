#!/bin/bash
set -e
pushd $(dirname ${BASH_SOURCE[0]})

for d in */build.sh ; do
    d=$(dirname "$d");
    echo building $d;
    $(cd "$d"; ./build.sh);
done

popd