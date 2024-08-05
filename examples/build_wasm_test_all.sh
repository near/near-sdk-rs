#!/bin/bash

BASE_DIR="./examples"
CARGO_VERSION = $1

for dir in "$BASE_DIR"/*; do
    if [ -d "$dir" ]; then
        cd "$dir" || exit

        echo "Building $dir"

        cargo nightly-aarch64-apple-darwin wasmcov build -- -Z build-std --all --target wasm32-unknown-unknown --release

        echo "Testing $dir"

        cargo +nightly-2024-05-01 test -- --workspace

        echo "Generating report"

        cargo +nightly-2024-05-01 wasmcov report

        cd - || exit
    fi
done
