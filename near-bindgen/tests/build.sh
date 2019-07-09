#!/bin/bash

pushd $(dirname "$0")/test-contract
wasm-pack build --no-typescript --release
wasm-opt -Oz --output ./pkg/optimized_contract.wasm ./pkg/test_contract_bg.wasm
cp pkg/optimized_contract.wasm ../res/test_contract.wasm
popd
