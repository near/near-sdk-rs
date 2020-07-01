#!/bin/sh
docker run \
     --mount type=bind,source=`pwd`/..,target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     -i -t contract-builder \
     /bin/bash

