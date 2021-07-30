#!/bin/bash

if [[ "${NEAR_RELEASE}" == "true" ]]; then
    echo "Test with release version of borsh and near-vm-logic"
    sed -n '/^borsh/p' near-sdk/Cargo.toml 
    sed -n '/^near-vm-logic/p' near-sdk/Cargo.toml
    cargo test --all
else
    echo "Test with git version of borsh and near-vm-logic"

    cargo generate-lockfile

    cp Cargo.toml{,.bak}
    cp Cargo.lock{,.bak}

    sed -i "" "s|###||g" Cargo.toml
    
    set +e
    cargo test --all
    status=$?
    set -e

    mv Cargo.toml{.bak,}
    mv Cargo.lock{.bak,}
    if [ $status -ne 0 ]; then
      exit $status
    fi

    # Only testing it for one configuration to avoid running the same tests twice
    echo "Build wasm32 for all examples"

    ./examples/build_all_docker.sh --check
    echo "Testing all examples"
    ./examples/test_all.sh
    # TODO re-enable when parsing unknown sections is fixed in twiggy 
    #      https://github.com/rustwasm/twiggy/pull/576
    # echo "Checking size of all example contracts"
    # ./examples/size_all.sh
fi
