#!/bin/bash
set -e

pushd $(dirname ${BASH_SOURCE[0]})

for d in "status-message" $(ls -d */ | grep -v -e "status-message\/$"); do
    pushd "$d"
    echo "Checking $d compiles for wasm32 target"
    RUSTFLAGS='-C link-arg=-s' cargo check --target wasm32-unknown-unknown --release
    popd
done

popd

