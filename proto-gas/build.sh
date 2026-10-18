#!/usr/bin/env bash
# Builds the three measurement wasms into proto-gas/res/.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/contract"
mkdir -p "$here/res"
for v in "v1:v1" "v2:v2_h16" "v2,h8:v2_h8"; do
  feat=${v%%:*}; name=${v##*:}
  cargo near build non-reproducible-wasm --no-abi --no-default-features --features "$feat" --out-dir "$here/res/tmp-$name"
  mv "$here/res/tmp-$name/proto_gas_contract.wasm" "$here/res/$name.wasm"
  rmdir "$here/res/tmp-$name"
done
ls -l "$here/res"
