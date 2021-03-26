#!/bin/bash
set -e

pushd $(dirname ${BASH_SOURCE[0]})

cargo install twiggy

for contract in */; do
    (
      cd "$contract";
      contract=$(basename "$contract")
      echo "Size contract $contract"
      RUSTFLAGS='-C debuginfo=2' cargo build --release --target wasm32-unknown-unknown
      for wasm in target/wasm32-unknown-unknown/release/*.wasm; do
        twiggy dominators -d 4 -r 100 "$wasm"
      done
    )
done

popd