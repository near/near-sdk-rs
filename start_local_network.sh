#!/usr/bin/env bash
#
# Starts locally running NEAR testnet. Launches several local nodes using the first node as the boot node and exposes
# the RPC API on 3030 port of the boot node which is bound to the 3030 port of the host machine.

set -e

IMAGE=${1:-nearprotocol/nearcore:0.1.7}
TOTAL_NODES=${2:-2}
NUM_ACCOUNTS=${3:-10}

sudo docker run -d --name testnet-0 -p 3030:3030 -p 26656:26656 --rm \
	-e "NODE_ID=0" \
	-e "TOTAL_NODES=${TOTAL_NODES}" \
	-e "NODE_KEY=53Mr7IhcJXu3019FX+Ra+VbxSQ5y2q+pknmM463jzoFzldWZb16dSYRxrhYrLRXe/UA0wR2zFy4c3fY5yDHjlA==" \
	-e "PRIVATE_NETWORK=y" \
	-e "NUM_ACCOUNTS=${NUM_ACCOUNTS}" \
	--rm \
	${IMAGE}

for NODE_ID in $(seq 1 `expr $TOTAL_NODES - 1`)
do
sudo docker run -d --name testnet-${NODE_ID} -p $((3030+${NODE_ID})):3030 -p $((26656+${NODE_ID})):26656 \
    --add-host=testnet-0:172.17.0.2 \
	-e "BOOT_NODES=6f99d7b49a10fff319cd8bbbd13c3a964dcd0248@172.17.0.2:26656" \
	-e "NODE_ID=${NODE_ID}" \
	-e "TOTAL_NODES=${TOTAL_NODES}" \
	-e "NUM_ACCOUNTS=${NUM_ACCOUNTS}" \
	-e "PRIVATE_NETWORK=y" \
	--rm \
	${IMAGE}
done
