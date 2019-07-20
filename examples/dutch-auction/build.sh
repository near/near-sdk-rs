#!/bin/bash

wasm-pack build --no-typescript --release
wasm-opt -Oz --output ./pkg/optimized_contract.wasm ./pkg/dutch_auction_bg.wasm
cp pkg/optimized_contract.wasm ./res/dutch_auction.wasm
rm -rf pkg
