#!/bin/bash
set -e

pushd $(dirname ${BASH_SOURCE[0]})

for d in */Cargo.toml ; do
    d=$(dirname "$d");
    echo "Testing $d";
    (cd $d && cargo test --workspace -- --nocapture)
done

popd
