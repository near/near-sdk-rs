#!/bin/bash
pushd ../examples
KEEP_NAMES=1 ./build_all.sh
popd
mkdir -p ./out/base/
for d in ../examples/*/ ; do
  cp $d/res/*.wasm ./out/base/
done

#for p in /work/near/core-contracts/*/res/*.wasm ; do
#  cp $p ./out/base/
# done

cd out
for p in ./base/*.wasm ; do
  w=$(basename -- $p)
  wasm-snip $p --snip-rust-fmt-code --snip-rust-panicking-code -p core::num::flt2dec::.* -p core::fmt::float::.*  \
     --output snipped-$w
  wasm-gc snipped-$w
  wasm-strip snipped-$w
  wasm-opt -Oz snipped-$w --output opt-snipped-$w
  cp $p stripped-$w
  wasm-strip stripped-$w
  echo $w `stat -c "%s" stripped-$w` " -> " `stat -c "%s" opt-snipped-$w`
done