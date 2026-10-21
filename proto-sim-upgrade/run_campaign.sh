#!/usr/bin/env bash
# The Oct 20 campaign: three searches of 512 cases, then the OutOfGas scan. Outputs in results/.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/sim"
export CARGO_TARGET_DIR=$HOME/Developer/near-contract-sim-scenario/target
for t in "search_from_v1 0x101" "search_from_v2a 0x102" "search_from_v2b 0x103"; do
  set -- $t
  NEAR_SIM_CASES=${CASES:-512} NEAR_SIM_SEED=$2 cargo +1.95.0 test --test storage_v2_upgrade "$1" -- --ignored --nocapture --exact > "$here/results/$1_$2.txt" 2>&1
  echo "$1 exit $?"
done
cargo +1.95.0 test --test storage_v2_upgrade scan_out_of_gas -- --ignored --nocapture --exact > "$here/results/scan_out_of_gas.txt" 2>&1
echo "scan exit $?"
