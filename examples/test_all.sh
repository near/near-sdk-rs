#!/bin/bash
set -e

pushd $(dirname ${BASH_SOURCE[0]})

if [[ $@ != *"--no-js"* ]]; then
    echo "Installing near-runner-jest"
    yarn install
    echo ""
fi

for d in */Cargo.toml ; do
    d=$(dirname "$d");
    echo "Testing $d";
    (cd $d && cargo test --workspace -- --nocapture)
    if [[ $@ != *"--no-js"* ]]; then
        if [ -d "$d/__tests__" ]; then
            (cd $d && yarn near-runner-jest)
        fi
    fi
done

popd
