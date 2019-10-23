#!/bin/bash
set -e

for d in */ ; do
    echo "Testing $d"
    pushd $d
    cargo test --features env_test
    popd
done