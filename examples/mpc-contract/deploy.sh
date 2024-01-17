#!/bin/bash
env NEAR_ENV=localnet near deploy "$1.node0" res/mpc_contract.wasm
