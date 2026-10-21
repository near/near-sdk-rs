# Storage v2 search on near-contract-sim

Storage v2's invariants run as a stateful property search on near-contract-sim: nearcore's
runtime in-process, with its real trie, real storage accounting and real gas. Results and
triage are in `PROTOTYPE_NOTES.md`, section "Sim search (Oct 20)".

- `contract/`: one v2 root. It holds a root scalar, `Map<AccountId, Account>` (where `Account`
  owns a nested `Map`, a `Vector` and a `Lazy<Vec<u8>>`), a `LookupMap` and a `Set`. Its methods
  cover the reference model's op set. Every mutating method takes a `mode`: plain, panic before
  the flush, `state_write()` then panic, or `state_write()` then a root change.
  `acct_remove_chain` drives `remove_step` through self-calls. `probe_has` calls the host
  directly, without the cache.
- `sim/tests/storage_v2_search.rs`: the harness. It replays every receipt the contract ran,
  in execution order, through a model. After every op it checks:
  - the whole trie, byte for byte, against the model;
  - `storage_usage`;
  - the `remove_step` budget and the clearing state;
  - every page, which must be exact or `CursorGone`;
  - that a failed op left no trace.
- `results/`: the Oct 20 run logs.

## Commands

The toolchain is 1.95.0, and `cargo` is the mbx shim. Reuse the simulator's target directory so
nearcore is not rebuilt:

```sh
cd proto-sim-search
./build.sh                                   # contract -> res/store.wasm (cargo-near 0.22)
cd sim
export CARGO_TARGET_DIR=~/Developer/near-contract-sim-scenario/target

# smoke: the hand-written witness through every op, and the positive control (planted orphan)
cargo +1.95.0 test --test storage_v2_search -- smoke positive_control

# search (256 cases by default; the Oct 20 runs below)
NEAR_SIM_SEED=0x20261020 NEAR_SIM_CASES=2000 cargo +1.95.0 test --test storage_v2_search -- search --exact --nocapture
SEARCH_GAS=wide NEAR_SIM_SEED=0x20261021 NEAR_SIM_CASES=2000 cargo +1.95.0 test --test storage_v2_search -- search --exact --nocapture

# every default fault at every receipt of two witnesses, plus caller-gas variants
cargo +1.95.0 test --test storage_v2_search -- scan_witnesses --nocapture

# OutOfGas at every host call: each step of a witness from 50 Ggas up in 20 Ggas steps
# (GAS_WALK_GGAS=n changes the stride)
cargo +1.95.0 test --test storage_v2_search -- gas_walk_every_step --nocapture
```

`NEAR_SIM_PROGRESS=0` silences the progress lines. Replays and leads go to
`$CARGO_TARGET_DIR/near-contract-sim/found/<test>/`.
