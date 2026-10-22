# proto-sim-upgrade: storage v2 upgrades, `Migrating` and `remove_step` on nearcore's runtime

A property-testing harness (near-contract-sim's `explore` API) for the
storage v2 features that span calls: redeploys with schema evolution, `Migrating<V1, V2>` with
`migrate_batch`, and `remove_step` clearing, under explored gas, block gaps, overlapping calls
and forced `OutOfGas` at self-call continuations. It runs on near-contract-sim main @ 8e8642db. It
was first written against b328d067 (`redesign/explore`, not main as this README said) and ported
on Oct 20. Results and triage: `PROTOTYPE_NOTES.md`, "Sim upgrade search (Oct 20)" and "Sim harness
port to primitives main (Oct 20)".

- `contract/`: one crate, five wasms (`--features v1|v2a|v2b|v2c|v3`): v1 Borsh root with a
  `store::IterableMap`; v2a = `Migrating<IterableMap, Map>` + `groups: Map<String, Group>`;
  v2b = Group schema changes (tag 3 removed + `reserved(3)`, `default = 7`, `default_fn`, tag 4
  re-used on purpose, new `flags: Set` at tag 8); v2c = v2b + a `required` tag (forbidden);
  v3 = the wrapper off. `drain` and `clear_loop` chain themselves with detached self-calls.
- `res/`: the built wasms. They are not committed (the root `.gitignore` drops `*.wasm`), so run
  `build.sh` first. `--no-abi` skips the ABI, so
  the `storage diff` that would refuse v2b's re-used tag and v2c's `required` never runs.
- `sim/tests/storage_v2_upgrade.rs`: the op enum, a model folded from every receipt's
  `ev v=<version> seq=<n>` log, the checks, hand tests, three searches and a scan. On main a
  finding is never excused (`Known` is gone). Each campaign therefore prints every finding and
  asserts it is one of three expected ones (`expected`): the re-used tag 4 (by design), the A3
  hole (open on ec29bf6f), and "unhandled failure" at a `clear_loop` or `drain` continuation
  that met a documented refusal after a redeploy. Anything else fails the test. `harness: seq
  has no gap` and `runtime: storage_usage ...` are sanity checks, not SDK properties. `probe:
  behaviour classes` never fails, and its counts include reruns. Distinct-case coverage is
  `Audit::reached` per method.
- `sim/tests/found/a3-orphaned-flags.json`: the scan's saved case for the A3 marker hole,
  converted to main's plan format. `a3_orphan_replay` replays it and must find the orphan on
  ec29bf6f. It is this harness's positive control.
- `results/`: `*_20261020b.txt` are the main runs, and the other files are the b328d067 runs.
  `run_campaign.sh` still writes the b328d067-era file names at 512 cases.

## Commands

`cargo` is the mbx shim; `cargo near` itself invokes the rustup toolchain's cargo.

```sh
cd proto-sim-upgrade
./build.sh                                   # all five wasms; VERSIONS="v2a v2b" ./build.sh for some
cd sim
# near-contract-sim main: ../../../near-contract-sim-main (see ../../proto-sim-search/README.md, Setup)
export CARGO_TARGET_DIR=~/Developer/near-contract-sim-scenario/target   # reuse the sim's nearcore build
cargo +1.95.0 test --test storage_v2_upgrade                            # hand tests, ~1 s
NEAR_SIM_CASES=2000 NEAR_SIM_SEED=0x102 cargo +1.95.0 test --test storage_v2_upgrade search_from_v2a -- --ignored --nocapture --exact
cargo +1.95.0 test --test storage_v2_upgrade scan_out_of_gas -- --ignored --nocapture --exact
cd .. && ./run_campaign.sh                   # b328d067-era: 3 x 512 cases + the scan, undated names; CASES=2000 to scale
```

The near-sdk mock pin: `NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features
unstable,unit-testing --test state_v2_verify hazard_a3_marker`.

## Timings

The b328d067 runs (Oct 20, debug profile, 512 cases, lightly loaded) took 145 to 200 ms per
case, and the scan 93 ms per variant. The main runs (`results/*_20261020b.txt`, `opt-level = 2`
for dependencies, 2,000 cases, machine at load 80 to 150) took 235 (v1), 152 (v2a) and 364 (v2b)
ms per case with shrinking excluded, and 9 to 20 min of wall time per search, most of it
shrinking the expected retag finding. Those runs are not comparable to the old ones because the
load differs. In the profile A/B on 128 fixed plans
(`timing_probe`), opt-level 0 took 479 and 620 ms per case and opt-level 2 took 189 and 214 ms,
on a machine at load 48 to 86 (`results/opt_level_ab_20261020b.txt`).
