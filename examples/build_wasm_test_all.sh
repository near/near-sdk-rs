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

echo "Merging .profraw files"
cargo wasmcov merge

cd $WASMCOV_DIR
mkdir lcov

for file in "$WASMCOV_DIR"/profdata/*.profdata; do
    profdata_file_name=$(basename "$file")
    # replacing '-' by '_' and changing extension .profdata by .o
    object_file_name=$(echo "$base_name" | sed 's/-/_/g' | sed 's/.profdata$/.o/')

    lcov_file_name=$(echo "$base_name" | sed 's/-/_/g' | sed 's/.profdata$/.lcov/')

    echo "Base profdata: $profdata_file_name"
    echo "Object file name: $object_file_name"
    echo "Lcov file name: $lcov_file_name"

    llvm-cov export --instr-profile= "$WASMCOV_DIR/profdata/$profdata_file_name" "$WASMCOV_DIR/target/$object_file_name" > "$WASMCOV_DIR/lcov/$lcov_file_name"
done


