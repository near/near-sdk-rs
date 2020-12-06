#!/bin/bash
# I think we don't do this cuz sim tests built it
set -e
cd "`dirname $0`"
source ../flags.sh
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/collection_comparison.wasm ./res/

