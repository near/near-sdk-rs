#!/bin/bash
set -e
cd "`dirname $0`"
source ../flags.sh
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/test_contract.wasm ./res/
#wasm-opt -Oz --output ./res/test_contract.wasm ./res/test_contract.wasm

