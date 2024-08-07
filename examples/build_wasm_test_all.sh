#!/bin/bash

for d in "status-message"  $(ls -d */ | grep -v -e "status-message\/$"); do
    if [ -d "$d" ]; then
      if [ "$d" = "wasmcov-output/" ]; then
          echo "Skipping $d"
          continue
      fi
      if [ "$d" = "wasmcov/" ]; then
          echo "Skipping $d"
          continue
      fi
      if [ "$d" = "near_sandbox/" ]; then
          echo "Skipping $d"
          continue
      fi
      echo building $d;
      cd $d;
      pwd
      ./build.sh true
      echo testing $d
      cargo-wasmcov test --near=1.40.0 --wasmcov-dir /Users/jrmncos/forks/near-sdk-rs/examples/wasmcov
      echo "Ending $d"
      cd ..
    fi
done


