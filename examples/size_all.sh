#!/bin/bash
set -e

cargo install twiggy

for contract in ./examples/*/; do
    (
      cd "$contract";
      contract=$(basename "$contract")
      echo "Size contract $contract"
      RUSTFLAGS='-C debuginfo=2' cargo build --release --target wasm32-unknown-unknown
      cp target/wasm32-unknown-unknown/release/*.wasm  res/
      for wasm in ./res/*.wasm; do
        twiggy dominators -d 4 -r 100 "$wasm"
      done
    )
done
