#!/bin/sh
docker buildx build --platform linux/arm64 -t contract-builder . --load
