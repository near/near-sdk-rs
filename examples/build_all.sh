#!/bin/bash
set -e

for d in */ ; do
    echo "Testing $d"
    pushd $d
    ./build.sh
    popd
done