#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-../../target}"
set -e
cd "$(dirname $0)"

cargo build --all --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/factory_contract_high_level.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/factory_contract_low_level.wasm ./res/
