# Storage v2 search on near-contract-sim

Storage v2's invariants run as a stateful property search on near-contract-sim: nearcore's
runtime in-process, with its real trie, real storage accounting and real gas. The harness runs on
near-contract-sim main @ 8e8642db ("One crate: primitives replaces main (#83)"). It was first
written against b328d067 (`redesign/explore`, the old explore layer) and ported on Oct 20. The
results, the port and the sim team's triage are in `PROTOTYPE_NOTES.md`, sections "Sim search
(Oct 20)" and "Sim harness port to primitives main (Oct 20)".

- `contract/`: one v2 root. It holds a root scalar, `Map<AccountId, Account>` (where `Account`
  owns a nested `Map`, a `Vector` and a `Lazy<Vec<u8>>`), a `LookupMap` and a `Set`. Its methods
  cover the reference model's op set. Every mutating method takes a `mode`: plain, panic before
  the flush, `state_write()` then panic, or `state_write()` then a root change.
  `acct_remove_chain` drives `remove_step` through self-calls. `probe_has` calls the host
  directly, without the cache.
- `res/store.wasm`: built by `build.sh` from this checkout's `near-sdk`. Not committed
  (`.gitignore`), so build it first.
- `sim/tests/storage_v2_search.rs`: the harness. It replays every receipt the contract ran, in
  execution order, through a model. The checks are SDK properties only:
  - at rest: the whole trie, byte for byte, against the model; every return value and panic;
    C5's clearing state and `remove_step` budget; C6's pages (exact or `CursorGone`); the raw
    `storage_has_key` probe under absent records;
  - per op, for `RemoveStep` and `RemoveChain` only: `remove_step` never adds a key under its
    record. When ops overlap it returns `Fail::not_judged`, never a pass.

  Two registered checks are not SDK properties, and their names say so. `runtime: storage_usage
  ...` is nearcore's accounting rule. `probe: behaviour classes` never fails. It counts receipts
  over every execution: shrink candidates, fault baselines, case 0's rerun and the faults line's
  reruns are all included, so its counts are upper bounds. Distinct-case coverage is
  `Audit::reached` per method. The positive control plants an orphan key with a harness-only
  op, `PlantOrphan`, and asserts that the model check finds it.
- `results/`: `*_20261020.txt` are the b328d067 runs, from the old API, and are not comparable
  line by line. `*_20261020b.txt` are the main runs. `opt_level_ab_20261020b.txt` is the
  profile A/B.

## Setup

The toolchain is 1.95.0, and `cargo` is the mbx shim. The harness depends on near-contract-sim
by the relative path `../../../near-contract-sim-main`. From `~/Developer/near-sdk-rs` that path
is `~/Developer/near-contract-sim-main`:

```sh
git -C ~/Developer/near-contract-sim-scenario worktree add ~/Developer/near-contract-sim-main 8e8642db
# A checkout under near-sdk-rs/.claude/worktrees/<name> resolves the path one level too deep:
ln -s ~/Developer/near-contract-sim-main ~/Developer/near-sdk-rs/.claude/worktrees/near-contract-sim-main
```

`sim/Cargo.toml` sets `opt-level = 2` for dependencies, as quickstart.md says. The simulator's
own `Cargo.toml` doesn't, so the first build compiles nearcore at opt-level 2 into the shared
target dir. That took 9 minutes, with mbx shared-cache hits. On a fixed set of 128 drawn plans,
opt-level 2 is about 2.8x faster: 338 and 371 ms per case at opt-level 0, against 127 and 124 ms
at opt-level 2, in two interleaved rounds on a loaded machine (`results/opt_level_ab_20261020b.txt`).

## Commands

```sh
cd proto-sim-search
./build.sh                                   # contract -> res/store.wasm (cargo-near 0.22)
cd sim
export CARGO_TARGET_DIR=~/Developer/near-contract-sim-scenario/target

# smoke: the hand-written witness through every op (refused: it panics on purpose), and the
# positive control (a planted orphan, found)
cargo +1.95.0 test --test storage_v2_search -- smoke positive_control --nocapture

# search (256 cases by default; the Oct 20 b runs below). Settle gaps, no repeats.
NEAR_SIM_SEED=0x20261020 NEAR_SIM_CASES=2000 cargo +1.95.0 test --test storage_v2_search -- search --exact --nocapture
SEARCH_GAS=wide NEAR_SIM_SEED=0x20261021 NEAR_SIM_CASES=2000 cargo +1.95.0 test --test storage_v2_search -- search --exact --nocapture

# every default fault at every receipt of two witnesses, plus the caller-gas and argument edges
cargo +1.95.0 test --test storage_v2_search -- scan_witnesses --exact --nocapture

# the caller's gas of each step of a 13-step witness, climbed by curves (replaces the gas walk)
cargo +1.95.0 test --test storage_v2_search -- caller_gas_curves --exact --nocapture

# profile A/B: the same TIMING_CASES (128) drawn plans, run one by one
cargo +1.95.0 test --test storage_v2_search timing_probe -- --ignored --exact --nocapture
```

`NEAR_SIM_PROGRESS=0` silences the progress lines. Replays go to
`$CARGO_TARGET_DIR/near-contract-sim/found/<test>/`.

Each search also prints what main's summary doesn't:
- the per-check judgement table (ok, failed, not judged, idle), read from `Audit::report()`'s
  JSON, because `Audit::judged` is crate-private at 8e8642db;
- `faults: N drawn, M landed`. The plan strategy is wrapped to keep each distinct plan it draws.
  After the search, each faulted plan runs once more through `Harness::run`, and its fault has
  landed if `Run::plan()` still carries it;
- the mean case time with shrinking excluded.
