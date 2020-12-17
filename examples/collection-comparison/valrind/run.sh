#!/bin/sh
HOST_DIR="$PWD/$(dirname ${BASH_SOURCE[0]})/../../.."
echo $HOST_DIR
docker run \
     --rm --mount type=bind,source=$HOST_DIR,target=/host \
     -i -t valgrind \
     /bin/bash