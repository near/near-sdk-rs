#!/bin/bash
set -e

if [[ $@ == *"--help"* || $@ == *"-h"* ]]; then
    echo "Run tests in all examples folders. Examples:"
    echo ""
    echo "    ./test_all.sh            # run only cargo tests"
    echo "    ./test_all.sh --with-js  # also run near-runner tests (requires NodeJS & Yarn)"
    echo ""
    exit 0
fi

pushd $(dirname ${BASH_SOURCE[0]})

if [[ $@ == *"--with-js"* ]]; then
    echo "Installing near-runner-jest"
    yarn install
    echo ""
fi

for d in */Cargo.toml ; do
    d=$(dirname "$d");
    echo "Testing $d";
    (cd $d && cargo test --workspace -- --nocapture)
    if [[ $@ == *"--with-js"* ]]; then
        if [ -d "$d/__tests__" ]; then
            (cd $d && yarn near-runner-jest)
        fi
    fi
done

popd
