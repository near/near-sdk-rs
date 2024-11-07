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
cp $TARGET/wasm32-unknown-unknown/release/adder.wasm ./res/
#wasm-opt -Oz --output ./res/status_message.wasm ./res/status_message.wasm