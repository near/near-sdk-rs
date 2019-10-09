#!/bin/bash

if [[ "${NEAR_RELEASE}" == "true" ]]; then
    echo "Test with release version of borsh and near-vm-logic"
    sed -n '/^borsh/p' near-bindgen/Cargo.toml 
    sed -n '/^near-vm-logic/p' near-bindgen/Cargo.toml
    cargo test --all
else
    echo "Test with git version of borsh and near-vm-logic"

    cp near-bindgen/Cargo.toml{,.bak}
    cp Cargo.lock{,.bak}

    sed -i "s|###||g" near-bindgen/Cargo.toml
    
    set +e
    cargo test --all
    status=$?
    set -e

    mv near-bindgen/Cargo.toml{.bak,}
    mv Cargo.lock{.bak,}
    exit ${status}
fi