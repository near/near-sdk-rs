#!/bin/bash
set -e
cd "`dirname $0`"
source ../flags.sh
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mission_control.wasm ./res/
#wasm-opt -Oz --output ./res/mission_control.wasm ./res/mission_control.wasm

