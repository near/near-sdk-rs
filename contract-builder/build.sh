#!/bin/bash
set -eox pipefail

branch=${BUILDKITE_BRANCH//:/_}
branch=${branch//\//_}
commit=${BUILDKITE_COMMIT}
if [[ ${commit} == "HEAD" ]]; then
    commit=$(git rev-parse HEAD)
fi

image_name="contract-builder"

docker buildx create --name contract-builder --use 
if [[ ${branch} == "master"]]; 
then
docker buildx build --platform linux/amd64,linux/arm64 -t nearprotocol/${image_name}:${branch}-${commit} -t nearprotocol/${image_name}:${branch} -t nearprotocol/${image_name}:latest --push .
else 
docker buildx build --platform linux/amd64,linux/arm64 -t nearprotocol/${image_name}:${branch}-${commit} -t nearprotocol/${image_name}:${branch} --push .
fi
