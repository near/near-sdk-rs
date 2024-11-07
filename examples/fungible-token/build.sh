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

$BUILD_COMMAND --all --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/defi.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/fungible_token.wasm ./res/