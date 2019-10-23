#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +nightly build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/cross_contract_high_level.wasm ./res/
#wasm-opt -Oz --output ./res/cross_contract_high_level.wasm ./res/cross_contract_high_level.wasm
rm -rf target
