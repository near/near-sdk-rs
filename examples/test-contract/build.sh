#!/bin/bash

if [ -n "$WASMCOV_DIR" ]; then
    TARGET="${WASMCOV_DIR}/target"
    BUILD_COMMAND="cargo wasmcov build -- --features wasmcov"
else
    TARGET="${CARGO_TARGET_DIR:-../../target}"
    BUILD_COMMAND="cargo build"
fi

set -e
cd "$(dirname $0)"
$BUILD_COMMAND --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/test_contract.wasm ./res/
#wasm-opt -Oz --output ./res/test_contract.wasm ./res/test_contract.wasm