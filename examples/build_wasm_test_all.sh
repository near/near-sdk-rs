#!/bin/bash

BASE_DIR="./examples"
CARGO_VERSION=$1

for dir in "$BASE_DIR"/*; do
    if [ -d "$dir" ]; then
        cd "$dir" || exit

        echo "Processing $dir"

        echo "Building $dir"

        cargo +$CARGO_VERSION wasmcov build -- -Z build-std=panic_abort,std --all --target wasm32-unknown-unknown --release

        echo "Testing $dir"

        cargo +$CARGO_VERSION wasmcov test -- --workspace

        echo "Generating report"

        cargo +$CARGO_VERSION wasmcov report

        ls -lha

        echo "Ending $dir"

        cd - || exit
    fi
done
