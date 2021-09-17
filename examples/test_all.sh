#!/bin/bash
set -e

pushd $(dirname ${BASH_SOURCE[0]})

echo "Installing near-runner-jest"
yarn install
echo ""

for d in */Cargo.toml ; do
    d=$(dirname "$d");
    echo "Testing $d";
    (cd $d && cargo test --workspace -- --nocapture)
    if [ -d "$d/__tests__" ]; then
        (cd $d && yarn near-runner-jest)
    fi
done

popd
