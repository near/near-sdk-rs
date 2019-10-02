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
    borsh_git='borsh = { git = "https://github.com/nearprotocol/borsh", branch = "master" }'
    vm_logic_git='near-vm-logic = { git = "https://github.com/nearprotocol/nearcore", branch = "staging" }'
    sed -i "s|^borsh.*|${borsh_git}|" near-bindgen/Cargo.toml
    sed -i "s|^near-vm-logic.*|${vm_logic_git}|" near-bindgen/Cargo.toml
    sed -n '/^borsh/p' near-bindgen/Cargo.toml 
    sed -n '/^near-vm-logic/p' near-bindgen/Cargo.toml
    
    set +e
    cargo test --all
    status=$?
    set -e

    mv near-bindgen/Cargo.toml{.bak,}
    mv Cargo.lock{.bak,}
    exit ${status}
fi