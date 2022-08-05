#!/bin/bash
set -e

pushd $(dirname ${BASH_SOURCE[0]})

for contract in */; do
    (
      cd "$contract";
      contract=$(basename "$contract")
      echo "Size contract $contract"
      for wasm in res/*.wasm; do
        du -hs "$wasm"
      done
    )
done

popd