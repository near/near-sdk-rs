#!/bin/bash


# Only testing it for one configuration to avoid running the same tests twice
echo "Build wasm32 for all examples"

./examples/build_all.sh
echo "Testing all examples"
./examples/test_all.sh
echo "Checking size of all example contracts"
./examples/size_all.sh


