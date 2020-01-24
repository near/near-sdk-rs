#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mission_control.wasm ./res/
#wasm-opt -Oz --output ./res/mission_control.wasm ./res/mission_control.wasm

