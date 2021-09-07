#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e
cd "`dirname $0`"
cargo build --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/cross_contract_high_level.wasm ./res/
#wasm-opt -Oz --output ./res/cross_contract_high_level.wasm ./res/cross_contract_high_level.wasm

