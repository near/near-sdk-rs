#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-../../target}"
set -e
cd "$(dirname $0)"

cargo build --all --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/defi.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/fungible_token.wasm ./res/
