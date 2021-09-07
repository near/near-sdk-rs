#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-target}"
set -e
cd "`dirname $0`"
cargo build --all --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/approval_receiver.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/non_fungible_token.wasm ./res/
cp $TARGET/wasm32-unknown-unknown/release/token_receiver.wasm ./res/