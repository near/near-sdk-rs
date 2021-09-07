#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e
cd "`dirname $0`"
cargo build --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/mission_control.wasm ./res/
#wasm-opt -Oz --output ./res/mission_control.wasm ./res/mission_control.wasm

