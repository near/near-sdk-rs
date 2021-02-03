#!/bin/sh

HOST_DIR="${HOST_DIR:-$(pwd)/..}"

docker run \
     --mount type=bind,source=$HOST_DIR,target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     -i -t contract-builder \
     /bin/bash

