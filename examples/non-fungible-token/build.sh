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
cp $TARGET/wasm32-unknown-unknown/release/approval_receiver.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/non_fungible_token.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/token_receiver.wasm ./res/