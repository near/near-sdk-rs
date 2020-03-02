#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/chatbot.wasm ./res/
#wasm-opt -Oz --output ./res/open_web.wasm ./res/open_web.wasm

