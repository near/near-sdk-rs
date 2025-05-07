#!/usr/bin/env bash
set -e
rustup target add wasm32-unknown-unknown

echo $(rustc --version)
pushd $(dirname ${BASH_SOURCE[0]})

declare -a example_dirs=("adder" 
                    "callback-results"
                    "cross-contract-calls/high-level"
                    "cross-contract-calls/low-level"
                    "factory-contract/high-level"
                    "factory-contract/low-level"
                    "fungible-token/ft"
                    "fungible-token/test-contract-defi"
                    "lockable-fungible-token"
                    "mission-control"
                )

for dir in "${example_dirs[@]}"; do
    echo '##################################'
    echo "building '$dir' ...";
    pushd $dir
    cargo near build non-reproducible-wasm
    popd
    echo "finished building '$dir' ...";
    echo '##################################'
done

popd

echo 'Build All Examples Finished!'
