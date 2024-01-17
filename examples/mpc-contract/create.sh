#!/bin/bash
env NEAR_ENV=localnet near create-account "$1.node0" --keyPath ~/.near/localnet/node0/validator_key.json --masterAccount node0
