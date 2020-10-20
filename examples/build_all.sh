#!/bin/bash
set -e

for d in */build.sh ; do
    d=$(dirname "$d");
    echo building $d;
    $(cd "$d"; ./build.sh);
done
