#!/usr/bin/env bash
set -e

echo $(rustc --version)
pushd $(dirname ${BASH_SOURCE[0]})

declare -a example_dirs=("adder"
                    "callback-results"
                    "cross-contract-calls"
                    "factory-contract"
                    "factory-contract-global"
                    "fungible-token"
                    "lockable-fungible-token"
                    "mission-control"
                    "mpc-contract"
                    "non-fungible-token"
                    "status-message"
                    "test-contract"
                    "versioned"
                )

for dir in "${example_dirs[@]}"; do
    echo '##################################'
    echo "testing '$dir' ..."
    pushd "$dir"

    cargo test --workspace

    popd
    echo "finished testing '$dir'"
    echo '##################################'
done


popd

echo 'Test All Examples Finished!\n'

echo 'Trying To Build All Examples'
./build_all.sh
