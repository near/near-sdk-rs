#!/usr/bin/env bash
set -ex

CHECK=0

pushd $(dirname ${BASH_SOURCE[0]})

# Loop through arguments and process them
for arg in "$@"
do
    case $arg in
        -c|--check)
        CHECK=1
        shift 
        ;;
    esac
done

for d in "status-message/"  $(ls -d */ | grep -v -e "status-message\/$"); do
	(./build_docker.sh ${d%%/})
done

if [ $CHECK == 1 ] && [ ! -z "$(git diff --exit-code)" ] ; then
	echo "Repository is dirty, please make sure you have committed all contract wasm files"
	exit 1
fi