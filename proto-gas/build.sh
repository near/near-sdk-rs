#!/usr/bin/env bash
# Builds the measurement wasms into proto-gas/res/. v2_h16_ext adds the methods for the rows
# added on Oct 18; the other three keep the Oct 17 method set, so their load cost is comparable.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/contract"
mkdir -p "$here/res"
for v in "v1:v1" "v2:v2_h16" "v2,h8:v2_h8" "ext:v2_h16_ext"; do
  feat=${v%%:*}; name=${v##*:}
  cargo near build non-reproducible-wasm --no-abi --no-default-features --features "$feat" --out-dir "$here/res/tmp-$name"
  mv "$here/res/tmp-$name/proto_gas_contract.wasm" "$here/res/$name.wasm"
  rmdir "$here/res/tmp-$name"
done
ls -l "$here/res"
