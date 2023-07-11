#!/bin/bash

set -e

cargo fmt --check --all
cargo clippy --tests --all-features -- -Dclippy::all
cargo test --all --features unstable,legacy
./examples/test_all.sh
