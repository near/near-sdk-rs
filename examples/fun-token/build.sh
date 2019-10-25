#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +nightly build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/fun_token.wasm ./res/
#wasm-opt -Oz --output ./res/fun_token.wasm ./res/fun_token.wasm
rm -rf target
