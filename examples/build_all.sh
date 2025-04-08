#!/bin/bash
set -e
rustup target add wasm32-unknown-unknown

echo $(rustc --version)
pushd $(dirname ${BASH_SOURCE[0]})

for d in "status-message"  $(ls -d */ | grep -v -e "status-message\/$"); do
    echo building $d;
    (cd "$d"; cargo near build non-reproducible-wasm; cp target/wasm32-unknown-unknown/release/*.wasm res/;);
    break;
done

popd
