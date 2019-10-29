#!/bin/bash
set -e

for d in */ ; do
    echo "Building $d"
    pushd $d
    ./build.sh
    popd
done
