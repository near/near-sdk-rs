#!/bin/bash
set -e

for d in "status-message" */ ; do
    echo "Building $d"
    pushd $d
    ./build.sh
    popd
done
