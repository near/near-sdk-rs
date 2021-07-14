#!/bin/bash

if [[ "${NEAR_RELEASE}" == "true" ]]; then
    echo "Test with release version of borsh and near-vm-logic"
    sed -n '/^borsh/p' near-sdk/Cargo.toml
    sed -n '/^near-vm-logic/p' near-sdk/Cargo.toml
    cargo test --all
else
    echo "Test with git version of borsh and near-vm-logic"

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

    ./examples/build_all.sh
    # echo "Testing all examples"
    # ./examples/test_all.sh
    # echo "Checking size of all example contracts"
    # ./examples/size_all.sh

    # After build_all, check for changes in wasm blobs and commit it to branch
    if ! git diff-index --quiet HEAD ./examples/**/*.wasm; then
      echo "--- Setting up git for pushing wasm blobs"
      git config user.name "BuildKite"
      git config user.email ${BUILDKITE_BUILD_AUTHOR_EMAIL}
      git remote set-url origin git@github.com:near/near-sdk-rs.git

      git fetch origin ${BUILDKITE_BRANCH}
      git checkout ${BUILDKITE_BRANCH}

      echo "--- Committing changes"
      git add ./examples/**/*.wasm
      git commit -m "Updated wasm blobs | build #${BUILDKITE_BUILD_NUMBER}"

      echo "--- Pushing to origin"
      git push -u origin ${BUILDKITE_BRANCH}
    else
      echo "No diff in wasm blobs found"
    fi
fi
