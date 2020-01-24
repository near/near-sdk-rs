#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/status_message_collections.wasm ./res/
#wasm-opt -Oz --output ./res/status_message_collections.wasm ./res/status_message_collections.wasm

