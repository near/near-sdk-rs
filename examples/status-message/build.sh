#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e
cd "`dirname $0`"
cargo build --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/status_message.wasm ./res/
#wasm-opt -Oz --output ./res/status_message.wasm ./res/status_message.wasm

