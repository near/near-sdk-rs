#!/bin/bash
set -e

cargo +nightly build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mission_control.wasm ./res/
#wasm-opt -Oz --output ./res/mission_control.wasm ./res/mission_control.wasm
wasm-gc ./res/mission_control.wasm
rm -rf target
