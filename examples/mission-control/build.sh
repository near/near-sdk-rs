#!/bin/bash
set -e

#wasm-pack build --no-typescript --release
#wasm-opt -Oz --output ./pkg/optimized_contract.wasm ./pkg/mission_control_bg.wasm
#cp pkg/optimized_contract.wasm ./res/mission_control.wasm
#rm -rf pkg

cargo +nightly build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mission_control.wasm ./res/
rm -rf target
