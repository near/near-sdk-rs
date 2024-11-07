#!/bin/bash

for d in "status-message"  $(ls -d */ | grep -v -e "status-message\/$"); do
    if [ -d "$d" ]; then
      echo "Processing directory $d"
      echo "Building"
      cd $d;
      pwd
      ./build.sh
      echo "Testing"
      cargo wasmcov test --near=1.40.0
      cd ..
      echo "End of processing of directory $d"
    fi
done




