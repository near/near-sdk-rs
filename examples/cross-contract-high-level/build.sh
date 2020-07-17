#!/bin/bash
set -e
cd "`dirname $0`"
source ../flags.sh
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/cross_contract_high_level.wasm ./res/
#wasm-opt -Oz --output ./res/cross_contract_high_level.wasm ./res/cross_contract_high_level.wasm

