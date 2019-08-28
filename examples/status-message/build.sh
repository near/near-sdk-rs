#!/bin/bash
set -e

cargo +nightly build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/status_message.wasm ./res/
wasm-opt -Oz --output ./res/optimized.wasm ./res/status_message.wasm
wasm-gc ./res/optimized.wasm
rm -rf target
