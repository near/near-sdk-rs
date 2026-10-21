#!/usr/bin/env bash
# Builds the search contract into proto-sim-search/res/store.wasm.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/contract"
mkdir -p "$here/res"
cargo near build non-reproducible-wasm --no-abi --out-dir "$here/res/tmp"
mv "$here/res/tmp/proto_sim_search_contract.wasm" "$here/res/store.wasm"
rmdir "$here/res/tmp"
ls -l "$here/res"
