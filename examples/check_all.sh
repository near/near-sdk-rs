#!/bin/bash
set -e

pushd $(dirname $0)

for d in */ ; do
    pushd $d
    echo "Checking $d compiles"
    RUSTFLAGS='-C link-arg=-s' cargo +nightly check --target wasm32-unknown-unknown --release
    popd
done

popd