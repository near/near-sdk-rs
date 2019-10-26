#!/bin/bash
set -e

pushd $(dirname $0)

for d in */ ; do
    pushd $d
    echo "Testing $d"
    cargo test -- --nocapture
    popd
done

popd
