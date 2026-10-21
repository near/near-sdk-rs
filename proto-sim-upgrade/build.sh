#!/usr/bin/env bash
# Builds the five wasms into proto-sim-upgrade/res/ (v1, v2a, v2b, v2c, v3).
# `--no-abi` skips the ABI, so the `storage diff` that should refuse v2b's re-used tag 4 and
# v2c's added `required` tag never runs: the sim sees exactly what that check would have caught.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/contract"
mkdir -p "$here/res"
for v in ${VERSIONS:-v1 v2a v2b v2c v3}; do
  cargo near build non-reproducible-wasm --no-abi --no-default-features --features "$v" --out-dir "$here/res/tmp-$v"
  mv "$here/res/tmp-$v/proto_sim_upgrade_contract.wasm" "$here/res/$v.wasm"
  rmdir "$here/res/tmp-$v"
done
ls -l "$here/res"
