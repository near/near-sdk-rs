#!/bin/bash

echo "Merging .profraw files"

cargo wasmcov merge
cd $WASMCOV_DIR
mkdir lcov
ls profdata

for file in "$WASMCOV_DIR"/profdata/*.profdata; do
    profdata_file_name=$(basename "$file")
    # replacing '-' by '_' and changing extension .profdata by .o
    object_file_name=$(echo "$profdata_file_name" | sed 's/-/_/g' | sed 's/.profdata$/.o/')

    lcov_file_name=$(echo "$profdata_file_name" | sed 's/-/_/g' | sed 's/.profdata$/.lcov/')

    echo "Base profdata: $profdata_file_name"
    echo "Object file name: $object_file_name"
    echo "Lcov file name: $lcov_file_name"
    echo "$WASMCOV_DIR/profdata/$profdata_file_name"
    echo "$WASMCOV_DIR/target/$object_file_name"
    echo "$WASMCOV_DIR/lcov/$lcov_file_name"
    echo "------------"
    llvm-cov export --instr-profile="$WASMCOV_DIR/profdata/$profdata_file_name" "$WASMCOV_DIR/target/$object_file_name" > "$WASMCOV_DIR/lcov/$lcov_file_name"
done