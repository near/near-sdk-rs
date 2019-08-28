#!/bin/bash
set -e

cargo +nightly build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/status_message.wasm ./res/
wasm-opt -Oz --output ./res/status_message.wasm ./res/status_message.wasm
wasm-gc ./res/status_message.wasm
rm -rf target
