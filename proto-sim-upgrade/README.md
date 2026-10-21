# proto-sim-upgrade: storage v2 upgrades, `Migrating` and `remove_step` on nearcore's runtime

A property-testing harness (near-contract-sim's `explore` API, sim main @ b328d067) for the
storage v2 features that span calls: redeploys with schema evolution, `Migrating<V1, V2>` with
`migrate_batch`, and `remove_step` clearing, under explored gas, block gaps, overlapping calls
and forced `OutOfGas` at self-call continuations. Results and triage: `PROTOTYPE_NOTES.md`,
"Sim upgrade search (Oct 20)".

- `contract/`: one crate, five wasms (`--features v1|v2a|v2b|v2c|v3`): v1 Borsh root with a
  `store::IterableMap`; v2a = `Migrating<IterableMap, Map>` + `groups: Map<String, Group>`;
  v2b = Group schema changes (tag 3 removed + `reserved(3)`, `default = 7`, `default_fn`, tag 4
  re-used on purpose, new `flags: Set` at tag 8); v2c = v2b + a `required` tag (forbidden);
  v3 = the wrapper off. `drain` and `clear_loop` chain themselves with detached self-calls.
- `res/`: the built wasms (gitignored; `build.sh` builds them). `--no-abi` skips the ABI, so
  the `storage diff` that would refuse v2b's re-used tag and v2c's `required` never runs.
- `sim/tests/storage_v2_upgrade.rs`: the op enum, a model folded from every receipt's
  `ev v=<version> seq=<n>` log, ten checks, hand tests, three searches and a scan.
- `sim/tests/found/a3-orphaned-flags.json`: the scan's saved case for the A3 marker hole, clean
  since round 7's first-write rule (`a3_orphan_replay` asserts the orphan finding is gone).
- `results/`: the Oct 20 campaign outputs. `run_campaign.sh` regenerates them.

## Commands

`cargo` is the mbx shim; `cargo near` itself invokes the rustup toolchain's cargo.

```sh
cd proto-sim-upgrade
./build.sh                                   # all five wasms; VERSIONS="v2a v2b" ./build.sh for some
cd sim
export CARGO_TARGET_DIR=~/Developer/near-contract-sim-scenario/target   # reuse the sim's nearcore build
cargo +1.95.0 test --test storage_v2_upgrade                            # hand tests, ~1 s
NEAR_SIM_CASES=512 NEAR_SIM_SEED=0x102 cargo +1.95.0 test --test storage_v2_upgrade search_from_v2a -- --ignored --nocapture --exact
cargo +1.95.0 test --test storage_v2_upgrade scan_out_of_gas -- --ignored --nocapture --exact
cd .. && ./run_campaign.sh                   # 3 x 512 search cases + the scan, ~6 min, into results/
```

The near-sdk mock pin: `NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features
unstable,unit-testing --test state_v2_verify a3_first_write`.

## Timings (Oct 20, debug test profile, M-series laptop, one search at a time)

| run | cases | wall | per case |
|---|---|---|---|
| `search_from_v1` (seed 0x101) | 512 | 74 s | 145 ms |
| `search_from_v2a` (seed 0x102) | 512 | 91 s | 177 ms |
| `search_from_v2b` (seed 0x103) | 512 | 102 s | 200 ms |
| `scan_out_of_gas` (8 witnesses, `Faults::default()`) | 480 | 45 s | 93 ms |
| hand tests (9) | | ~1 s | |

`results/` was regenerated after round 7 (the A3 first-write rule): 97 s, 88 s, 114 s and 51 s,
with other jobs on the machine.
The scan's "no collection outlives its record" failures went from 50 to 0, and its A3 removal
refusals from 42 to 137; the only Knowns left are the re-used tag 4 and continuation refusals.
