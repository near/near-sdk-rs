#!/bin/bash
set -e
rustup target add wasm32-unknown-unknown

echo $(rustc --version)
pushd $(dirname ${BASH_SOURCE[0]})

for d in "status-message"  $(ls -d */ | grep -v -e "status-message\/$"); do
    for directory in $(find $d -type d); do
        if [ -d "$directory/src" ]; then
            echo building $d;
            (cd "$d"; cargo near build non-reproducible-wasm;);
        fi
    done
done

popd
