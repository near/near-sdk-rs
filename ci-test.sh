#!/bin/bash

if [[ "${NEAR_RELEASE}" == "true" ]]; then
    echo "Test with release version of borsh and near-vm-logic"
    sed -n '/^borsh/p' near-bindgen/Cargo.toml 
    sed -n '/^near-vm-logic/p' near-bindgen/Cargo.toml
    cargo test --all
else
    echo "Test with git version of borsh and near-vm-logic"

    cp Cargo.toml{,.bak}
    cp Cargo.lock{,.bak}

    sed -i "s|###||g" Cargo.toml
    
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
    echo "Checking compilation of wasm32 for all examples"
    ./examples/check_all.sh
    echo "Testing all examples"
    ./examples/test_all.sh
fi

