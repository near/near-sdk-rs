#!/bin/bash
set -e

rm -rf wasm_sizer
git clone https://github.com/near/wasm_sizer
pip3 install --user numpy matplotlib

for contract in $(ls examples/*/res/*.wasm); do
    echo "Size contract $contract"
    python3 wasm_sizer/sizer.py --input $contract --sections --silent
done
