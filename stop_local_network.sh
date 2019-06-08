#!/usr/bin/env bash
#
# Removes locally running NEAR testnet.

set -e
sudo docker stop $(sudo docker ps -f name=testnet -q)
