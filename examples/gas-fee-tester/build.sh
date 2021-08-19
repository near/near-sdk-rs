#!/bin/bash
set -e
TARGET="${CARGO_TARGET_DIR:-target}"
cd "`dirname $0`"
cargo build --target wasm32-unknown-unknown --release
cp $TARGET/wasm32-unknown-unknown/release/gas_fee_tester.wasm ./res/

