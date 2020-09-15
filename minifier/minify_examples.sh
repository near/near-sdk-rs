#!/usr/bin/env bash
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
  ../minify.sh $p
  cp $p stripped-$w
  wasm-strip stripped-$w
  echo $w `stat -c "%s" stripped-$w` " -> " `stat -c "%s" minified-$w`
done