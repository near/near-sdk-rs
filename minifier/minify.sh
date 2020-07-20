#!/usr/bin/env bash

for p in "$@"; do
  w=$(basename -- $p)
  echo "Minifying $w, make sure it is not stripped"
  wasm-snip $p --snip-rust-fmt-code --snip-rust-panicking-code -p core::num::flt2dec::.* -p core::fmt::float::.*  \
     --output temp-$w
  wasm-gc temp-$w
  wasm-strip temp-$w
  wasm-opt -Oz temp-$w --output minified-$w
  rm temp-$w
  echo $w `stat -c "%s" $p` "bytes ->" `stat -c "%s" minified-$w` "bytes, see minified-$w"
done