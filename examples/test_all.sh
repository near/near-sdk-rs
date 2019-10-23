#!/bin/bash
set -e

pushd $(dirname $0)

for d in */ ; do
    echo "Testing $d"
    pushd $d
    RUSTFLAGS='-C link-arg=-s' cargo +nightly check --target wasm32-unknown-unknown --release
    cargo test --features env_test -- --nocapture
    popd
done

popd