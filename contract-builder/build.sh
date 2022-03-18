#!/bin/bash
set -eox pipefail

if [ $1 != "linux/amd64"] || [$1 != "linux/arm64"]; then 
  echo " Please enter one of linux/amd64 or linux/arm64"
  exit 1
fi



branch=${BUILDKITE_BRANCH//:/_}
branch=${branch//\//_}
commit=${BUILDKITE_COMMIT}
if [[ ${commit} == "HEAD" ]]; then
    commit=$(git rev-parse HEAD)
fi

image_name="contract-builder"

if docker buildx ls| grep -q contract-builder;then
    docker buildx use --builder contract-builder
else
    docker buildx create --name contract-builder --use 
fi

if [[ ${branch} == "master" ]];then
    docker buildx build --platform $1 -t nearprotocol/"${image_name}:${branch}-${commit}-$1" -t nearprotocol/${image_name}:latest-$1 --push .
else 
    docker buildx build --platform $1 -t nearprotocol/"${image_name}:${branch}-${commit}-$1" --push .
fi
