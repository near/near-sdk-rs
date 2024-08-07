#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-../../target}"
set -e
cd "$(dirname $0)"

INCLUDE_COVERAGE="${1:-false}"

if [ "$INCLUDE_COVERAGE" = "true" ]; then
  cargo-wasmcov build  --wasmcov-dir /Users/jrmncos/forks/near-sdk-rs/examples/wasmcov -- --all --target wasm32-unknown-unknown --release
  cp ../wasmcov/target/wasm32-unknown-unknown/release/lockable_fungible_token.wasm ./res/

else
  cargo build --target wasm32-unknown-unknown --release
  cp $TARGET/wasm32-unknown-unknown/release/lockable_fungible_token.wasm ./res/
fi