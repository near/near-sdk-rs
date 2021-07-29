#!/usr/bin/env bash

# Exit script as soon as a command fails.
set -ex

NAME="$1"
IDENT=${NAME#*/}
echo "$IDENT"

# Switch to current directory (./examples) then out to root for specific examples
pushd $(dirname ${BASH_SOURCE[0]})
cd ../

if docker ps -a --format '{{.Names}}' | grep -Eq "^build_${IDENT}\$"; then
    echo "Container exists"
else
docker create \
     --mount type=bind,source=$(pwd),target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     --name=build_$IDENT \
     -w /host/$NAME \
     -e RUSTFLAGS='-C link-arg=-s' \
     -it nearprotocol/contract-builder \
     /bin/bash
fi

docker start build_$IDENT
docker exec build_$IDENT /bin/bash -c "./build.sh"

# mkdir -p res
# cp $NAME/target/wasm32-unknown-unknown/release/$CONTRACT_WASM_NAME.wasm $NAME/res/$CONTRACT_WASM_NAME.wasm
