# Storage v2 prototype: gas measured on the real runtime

## Oct 18, 2026 re-measurement, after API rounds 2-5

Branch `proto/storage-v2-gas2` from `proto/storage-v2` @ `5030854b` (API round 5). Same simulator
(near-contract-sim, nearcore 2.14.0-rc.2, protocol 87, no source change since Oct 17), same
harness, same key counts and keys. Raw output: `proto-gas/results/*_20261018.txt`.

The harness (`proto-gas/sim`) depends on near-contract-sim and is not on this branch. It is kept
on the local branch `proto/storage-v2-sim-archive`; the contract, `build.sh` and the raw
results stay here.

**Headline.** No storage-op count or trie-node count moved on any Oct 17 row, and byte counts moved
only on `Map` removal (below). Every
v2 row costs 6-30 Ggas more for exactly two reasons: a 5.7 KB larger wasm (+6.2 Ggas of contract
loading on every v2 call), and more wasm instructions in the SDK's cache and flush code, most of it
from the D9 flag-key change (`d4f2846c`). The flag-key *check*, the kind markers and the 12-byte
header add no host call to these rows: their maps hold leaf values, `balance_of` goes through
`at`, and the root carries no markers. The one storage-level change is +8 bytes on the header
write of a `Map` removal (12-byte header once `revision > 0`), about 0.5 Ggas. v1 is unchanged.

The commit attribution is measured (bisection below). The mechanism comes from reading the D9
diff, not from measurement: `flush_ops` now builds a `Vec<FlushOp>` with copied keys, and
re-encodes and compares each dirty slot, for every key whether flaggable or not. That is why
leaf-only rows pay it with no extra host call. The `Map::try_insert` record row's +10 Ggas over
a leaf insert compares two different builds (ext vs non-ext), so codegen differences are not
excluded. It is consistent with the cost of encoding the record.

*Update, same day:* measured and fixed in "Flush regression fix (Oct 18, 2026)" below. The
copies were the cause, together with a key copy before every host read in `Cache::load`. The
"re-encodes every dirty slot" part does not hold for the wasm path. With the fix, all six
write-path rows are back at or below their Oct 17 op-only gas.

### Method changes

- **The "before" is a rebuild.** The Oct 17 wasms were not kept. `9382156c` (the Oct 17 branch head)
  rebuilt with the same toolchain reproduces `gas_report.txt` in every column except contract
  loading, 0.1 Ggas higher on v2 rows (13 of 71 Tgas cells move by 0.001). The cause is wasm size:
  `#[track_caller]` panic locations embed the source path, and the rebuild sat in a longer
  directory. So the Oct 17 table below is exact to ±0.001 Tgas, and the deltas come from the
  rebuild (`gas_report*_oct17rebuilt_20261018.txt`).
- **Two contract builds.** The old matrix runs on `v1`, `v2_h16`, `v2_h8` built from the Oct 17
  method set under the current API. Its source needed no edit: it never called `insert_new` or
  `remove_some`. The new rows need five more methods, which add 25.6 KB of wasm and would add
  ~28 Ggas of loading to every old row. They live behind a new `ext` feature (`v2_h16_ext.wasm`),
  measured only for the new rows and against that wasm's own `ping`.
- **Bisection.** To attribute the wasm-instruction growth, the non-ext contract was built against
  17 commits from `9382156c` to `b4c69bdb` and measured on a 1k-key subset (`bisect_rows`,
  `GAS_RES_DIR`; output `gas_bisect_20261018.txt`).
- **Harness hazard found.** Two copies of the sim harness (two worktrees) sharing one
  `CARGO_TARGET_DIR` build to the same test binary name. Running them at the same time makes one
  silently run the other's binary and wasm paths. Run one at a time.

### Wasm size

| build | Oct 17 (rebuilt at `9382156c`) | Oct 18 (`5030854b`) | Δ |
|---|---:|---:|---:|
| v1 | 136,429 | 136,365 | −64 B |
| v2 h16 | 158,796 | 164,515 | **+5,719 B (+3.6%)**, load 173.0 → 179.2 Ggas |
| v2 h8 | 158,781 | 164,493 | +5,712 B |
| v2 h16 `ext` (new rows only) | n/a | 190,119 | |

v2 is now 28.2 KB larger than v1 (was 22.4 KB), i.e. 30.6 Ggas more load per call (was 24.4).
The growth per step: API round 1 +1.0 KB, D9 flag key +1.5 KB, T4 revision +1.3 KB, the rest of
round 3 +0.7 KB, A2 generation stamps +1.1 KB, A3 markers +0.1 KB.

### Before / after (Tgas, whole receipt; op-only = minus the same wasm's `ping`)

Columns: 1k / 10k / 100k keys / 100k keys + 100k accounts. Only the h16 rows are shown. h8 moved
by the same amounts (±1 Ggas).

| row | Oct 17 | Oct 18 | Δ whole / op-only (Ggas) | verdict |
|---|---|---|---:|---|
| v2 `Map` insert new | 1.789 / 1.794 / 1.801 / 1.855 | 1.808 / 1.813 / 1.820 / 1.874 | +19 / +13 | **changed**: wasm size +6.2; wasm instructions +12.7 (D9 flush +10.4, A2 stamps +1.4, rest +0.9). Ops 3/4/0/0 unchanged, no flag-key check (leaf), header still 4 B |
| v2 `Map` update existing | 1.417 / 1.419 / 1.422 / 1.435 | 1.428 / 1.431 / 1.433 / 1.447 | +11-12 / +5 | **changed**: wasm size +6.2; instructions +5.1 (D9 +3.9, A2 +0.7) |
| v2 `Map` remove non-last | 2.138 / 2.145 / 2.159 / 2.241 | 2.168 / 2.175 / 2.189 / 2.271 | +30 / +24 | **changed**: wasm size +6.2; instructions +23.1 (D9 +16.2, T4 revision +4.3, A2 +1.8, rest +0.8); header bytes +0.5 (12-byte header). Ops 5/3/3/0 unchanged |
| v2 `LookupMap` insert new | 1.309 / 1.312 / 1.314 / 1.328 | 1.319 / 1.321 / 1.324 / 1.337 | +9-10 / +3 | **changed**: wasm size +6.2; instructions +3.3 (D9 +2.2, A2 +0.7) |
| v2 `LookupMap` update | 1.318 / 1.320 / 1.322 / 1.336 | 1.327 / 1.330 / 1.332 / 1.346 | +9-10 / +3 | **changed**: same causes as insert |
| v2 `LookupMap` remove | 1.372 / 1.374 / 1.376 / 1.390 | 1.383 / 1.385 / 1.388 / 1.401 | +11 / +5 | **changed**: wasm size +6.2; instructions +5.2 (D9 +4.1, A2 +0.7) |
| v1 `IterableMap` / `LookupMap`, all ops | | identical | 0 (±1) | **unchanged within noise** (v1 wasm −64 B, load still 148.6) |
| remove beside 1 B / 100 KiB, v2 `Map` | 2.116 / 2.116 (2.198 with 100k accts) | 2.146 / 2.146 (2.228) | +30 / +24 | **changed**, causes as `Map` remove. Neighbor independence holds: identical receipts |
| remove beside 1 B / 100 KiB, v1 | 1.910 / 10.745 (10.813) | 1.910 / 10.745 | 0 | **unchanged** |
| `ping` | 1.061 | 1.067 | +6 | **changed**: wasm size only |
| `noop_view` (root read) | 1.150 | 1.157 | +7 / +1 | **changed**: wasm size; decode +1.2. Root read 89 → **90 Ggas** |
| `noop_mut` (unchanged `&mut self`) | 1.176 | 1.184 | +8 / +2 | **changed**: wasm size; +1.5 instructions. Still 1/0/0/0: **writes nothing, holds** |
| `balance_of` (nested read) | 1.288 | 1.298 | +10 / +4 | **changed**: wasm size; +3.4 instructions (D9 +2.8). Ops 2/0/0/0; `at` still checks nothing. Nested read 138 → **141 Ggas** |

None of these deltas is noise: the simulator is deterministic, and the same keys moved by the
same amount (the 7-key spreads are unchanged). Measured against the 5-9 Ggas spread between keys
that the Oct 17 run used as its noise scale, the op-only deltas are within it except `Map` insert
(+13) and `Map` remove (+24). The whole-receipt deltas also carry the 6.2 Ggas of load.

**`Map` premium on a new key, recomputed:**

| trie | Map − LookupMap (v2) | Map (v2) − IterableMap (v1) | op-only Map : LookupMap |
|---|---:|---:|---:|
| 1k keys | +0.489 Tgas (was 0.480) | +0.302 (was 0.283) | 0.74 : 0.25 (2.9x) |
| 100k keys | +0.496 (was 0.487) | +0.305 (was 0.286) | 0.75 : 0.26 (2.9x) |
| 100k + 100k accounts | **+0.537 (was 0.527)** | +0.318 (was 0.299) | 0.81 : 0.27 (3.0x) |

Breakdown of the 0.537 at 100k+100k: 2 extra reads 113 Ggas and 3 extra writes 193 Ggas of base
(305 together, unchanged), trie nodes 112 (49 nodes, unchanged), sha256 10, per-byte 6.5, wasm
**46 (was 37)**, other 57. The whole +10 is wasm instructions; storage is byte-identical.
Against v1 `IterableMap` the premium grows more, +19, because v1 pays neither the load nor the
instruction growth.

### New rows (v2 h16, `ext` wasm 190,119 B, `ping` 1.095 Tgas)

| row | 1k | 10k | 100k | 100k + 100k accts | op-only | r/w/d/h | notes |
|---|---:|---:|---:|---:|---:|---|---|
| `page_with(Some(after), 50)` of `Map<AccountId, u128>` | 10.771 | 10.769 | 10.774 | 10.774 | 9.68 | 103/0/0/0 | root + cursor index + header + 50 slots + 50 entries. Base 5.80 Tgas, wasm 1.70, sha256 0.26 (one hash per entry path), other 2.70 (0.89 call floor, the rest register and return of the JSON page). Reads charge no trie nodes, so it does not grow with the trie. ~0.19 Tgas per entry |
| `Map::try_insert` of `Account { nonce, balances: Map }` (new key) | 1.846 | 1.851 | 1.857 | 1.912 | 0.75-0.82 | 3/4/0/0 | Same ops as a leaf `Map::insert`. Op-only is +10 Ggas over the leaf insert, all wasm (record encode). The 13-byte record includes the 3-byte kind marker. No flag-key check: the index read decides |
| `LookupMap::try_insert` of the same record | 1.474 | 1.474 | 1.474 | 1.488 | 0.38-0.39 | 1/1/0/2 | The 2nd `storage_has_key` is the D9 flag-key check on a miss (~54 Ggas base). +0.127 Tgas op-only over a leaf `LookupMap::insert`: 2 has_key 108 Ggas, wasm 12, trie nodes |
| `remove_step(n = 3)`, first call, record + 10-entry map | 1.764 | 1.769 | 1.774 | 1.801 | 0.67-0.71 | 4/1/1/0 | Removes **no entry**: it only relocates the record to its flag key (1 remove + 1 write). The move is reserved first, which leaves 2 removes, less than one entry (3) |
| `remove_step(n = 3)`, finishing call | 2.115 | 2.120 | 2.129 | 2.184 | 1.02-1.09 | 6/1/3/0 | Record (at the flag key), index and last slot removed; parent header written |
| `remove_step(n = 3)`, whole removal | 27.87 (13 calls) | 27.98 | 28.08 | 28.73 | | 35 removes | 1 relocation call, 10 calls of 1 entry (3 removes) each, 1 for the emptied map's header, 1 finishing call (3 removes) |
| `remove_step(n = 64)`, one call, record + 10-entry map | 7.672 | 7.748 | 7.827 | 8.306 | 6.6-7.2 | 15/1/34/0 | Trie nodes 597-875 (1.36-2.0 Tgas), wasm 1.60 Tgas |

At the minimum budget the whole removal costs 3.6x the one-call removal, because each of its 13
calls pays the ~1.1 Tgas call floor plus the root and record reads. `MIN_REMOVE_STEP = 3`
guarantees progress. It is not a sensible operating point: a budget sized to the gas a call can
spend is.

### Figures in the design doc (appendix C) that must change

| doc quotes | now | change? |
|---|---|---|
| `Map` new key +0.5 Tgas over `LookupMap` | +0.49 / +0.50 / +0.54 | "~0.5" holds. Exact figure 0.527 → **0.537** |
| removal v1 1.91 / 10.75 vs v2 2.12 Tgas | v1 1.91 / 10.75 vs v2 **2.15** (2.23 with 100k accounts; op-only 1.08) | **yes**, v2 figure |
| root read 89 Ggas | **90** | yes (+1, decode instructions) |
| nested read 138 Ggas | **141** | yes (+3, mostly D9 cache code) |
| unchanged `&mut self` call writes nothing | 1 read, 0 writes | holds |
| 0.527 Tgas premium decomposition | 0.537: storage terms identical, wasm 37 → 46 | **yes**, total and wasm term |
| v2 wasm 23 KB larger than v1, ~24 Ggas load (limits section below) | 28 KB, ~31 Ggas | yes |
| `Map` over v1 `IterableMap` +0.28-0.30 | +0.30-0.32 | yes |

### Not measured

- No v1 counterpart for the new rows (v1 has no cursor page and no bounded removal); no h8 run of
  the new rows.
- `#[key(n, legacy)]`, `at_mut` (present or absent record) and a first `deposit` (the flag-key read
  on a record miss through `entry`) have no row. The flag-key check is visible only in
  `LookupMap::try_insert`. A `Map` miss through `try_insert` never checks it, because the index
  decides.
- The old records written before round 5 (no markers) were not measured against new ones.
- The original Oct 17 wasm bytes; see the rebuild note above.
- The design doc's appendix C itself (the Claude Doc was not available to this session); the figures
  above are the ones quoted in the brief.

### Flush regression fix (Oct 18, 2026)

Branch `proto/storage-v2-perf` (from `5030854b` plus the gas harness, `cb867b4f`). Fix commit
`19050adb`. Same simulator and harness. The rows are the six v2 h16 write-path rows at 10k keys,
run by the new `perf_rows` helper (same `ops()` as `gas_report`, 7 keys per row, median). Raw
output: `proto-gas/results/gas_flushfix_20261018.txt`. The sim ran from its own
`CARGO_TARGET_DIR` (`.sim-target/` in this worktree), so the shared-binary hazard above could not
arise.

**Cause, measured.** The D9 flag-key commit added a heap allocate + copy + free in two places,
and those two places account for the whole D9 delta:

1. **Read path.** `Cache::load` copied the key before every host read (`key.to_vec()`, even when
   the slot was not at its flag key). Each read also looked up the slot twice (`entry_slot`, then
   `load`).
2. **Flush.** `Slot::flush_ops` returned a `Vec<FlushOp>` per slot, with a copied key in every
   op.

Separately, the flush cloned every slot key before its loop (`keys: Vec<(Vec<u8>, _)>`). That
cost predates Oct 17 and is not part of the regression. It was removable in the same change.

One variant per cause, each built alone on the baseline, measured as WASM_INSTRUCTION Ggas
(1 Ggas ≈ 1,215 wasm ops at nearcore's `wasm_regular_op_cost` of 822,756 gas):

| row (10k keys) | D9 delta in the bisect (1k keys) | (a) read path only | (b) flush-op visitor only | (a)+(b) | (c) keys walked in place only | all three |
|---|---:|---:|---:|---:|---:|---:|
| `Map` insert new | +10.4 | −4.0 | −6.3 | −10.3 | −5.8 | −15.6 |
| `Map` update existing | +3.9 | −2.3 | −1.6 | −3.9 | −3.8 | −7.8 |
| `Map` remove non-last | +16.2 | −8.3 | −9.0 | −17.3 | −6.8 | −24.1 |
| `LookupMap` insert new | +2.2 | −0.7 | −1.4 | −2.1 | −3.0 | −5.3 |
| `LookupMap` update | +2.2 | −0.7 | −1.5 | −2.2 | −3.0 | −5.3 |
| `LookupMap` remove | +4.1 | −2.6 | −1.5 | −4.1 | −3.0 | −7.2 |

(a)+(b) reproduces the bisect's D9 delta on every row to within 1.1 Ggas. The savings add up:
(a)+(b)+(c) taken separately is within 0.5 Ggas of the combined build. Per extra
allocate/copy/free that is about 0.7-1.7 Ggas (850-2,000 wasm ops): the allocator's
malloc and free, plus the copy of a 40-60 byte key. No storage op, byte or trie-node
count moved in any variant.

**The re-encode half of the earlier hypothesis is refuted for the wasm path.** Both before and
after D9 the flush encodes each Touched/Dirty slot exactly once. The second encode is in
`has_pending_writes`, which also calls the op builder, but only `state::testing` (the view-call
check) calls it, never a contract. The copy half was right, and the read-path copy in `load` was
the other half.

**The change** (`near-sdk/src/state/cache.rs`):

- `Cache::load(key, flaggable)` marks the slot and loads it with one slot lookup. It passes the
  key to the host as is and builds the flag key only when the slot is known to be at it.
- `Slot::flush_ops` → `Slot::for_each_flush_op(f)`, a visitor. Each `FlushOp<'_>` borrows the
  encoded bytes and names its target as `At::Key` or `At::Flag`. The flush builds the flag key
  only for an op that targets it (clearing moves, blind deletes of records). A verbatim
  clearing move borrows the original bytes (`Cow`) instead of copying them.
- `flush` walks the slot map in place with disjoint field borrows (`host`, `slots`, `root`) and
  compares the root key as a slice. Nothing in the loop adds a slot, and a host call cannot
  re-enter the cache, because its `RefCell` is borrowed for the whole flush.
- Same ops in the same order: the record key before its flag key, and key order overall. Same
  counts: `remove_count` still matches the visitor (the `pending_removes` debug assert). No new
  `unsafe`: the flush keeps its existing `sp.as_ref()` deref.

**Before / after** (Tgas whole receipt; op-only = minus the same wasm's `ping`, which removes the
contract-loading difference):

| row (10k keys) | Oct 17 (`9382156c` rebuilt) | Oct 18 before fix | after fix | op-only Oct 17 / before / after (Ggas) | op-only after − Oct 17 | WASM_INSTRUCTION Oct 17 / before / after (Ggas) |
|---|---:|---:|---:|---:|---:|---:|
| `Map` insert new | 1.794 | 1.813 | **1.796** | 732.7 / 745.3 / 729.8 | **−2.9** | 96.4 / 109.1 / 93.5 |
| `Map` update existing | 1.419 | 1.431 | **1.422** | 358.3 / 363.3 / 355.6 | **−2.7** | 70.9 / 76.0 / 68.2 |
| `Map` remove non-last | 2.145 | 2.175 | **2.150** | 1084.2 / 1107.9 / 1083.8 | **−0.4** | 117.1 / 140.2 / 116.1 |
| `LookupMap` insert new | 1.312 | 1.321 | **1.315** | 250.8 / 254.0 / 248.7 | **−2.1** | 59.7 / 63.0 / 57.7 |
| `LookupMap` update | 1.320 | 1.330 | **1.323** | 259.1 / 262.3 / 257.0 | **−2.1** | 60.0 / 63.3 / 58.0 |
| `LookupMap` remove | 1.374 | 1.385 | **1.377** | 312.9 / 318.1 / 310.9 | **−2.0** | 59.9 / 65.1 / 57.9 |

Ops are unchanged on every row: 3/4/0/0, 2/1/0/0, 5/3/3/0, 1/1/0/0, 1/1/0/0, 2/0/1/0.

**Wasm size** (`v2_h16`, built in this worktree): Oct 17 rebuilt 158,806 B, before fix 164,513 B,
after 163,361 B (**−1,152 B**). Contract loading is now 178.0 Ggas (was 179.2; Oct 17 173.0).
So the whole-receipt gap to Oct 17 is +2 to +5 Ggas, which is the remaining load delta (+5.0)
minus the op-only gain. The variant builds were (a) 164,509 B, (b) 164,050 B and (c) 163,834 B.
The combined build was 163,328 B before comment-only edits. Those edits shift the source lines
that `#[track_caller]` panic locations embed, which accounts for +33 B. The build is
deterministic (same SHA-256 twice).

**Target (within 3 Ggas of Oct 17 after the loading delta): met on all six rows.** Op-only is
0.4-2.9 Ggas *below* Oct 17. That is not because the post-Oct-17 instruction growth is gone:

- **D9 flag key, inherent part: about 0 on these rows.** (a)+(b) recover the whole D9 delta
  (within 0.1 Ggas on five rows, and 1.1 Ggas more than the D9 delta on `Map` remove). These rows store leaf values, so
  they never take a flag-key branch. The remove ordering (record key before flag key) is a
  branch taken only for records being cleared, and it adds no host call here. The one D9 host
  cost, the flag-key check on a record miss (`LookupMap::try_insert`, about 54 Ggas base), is
  inherent to the design and untouched.
- **Later commits, still present:** with only (a)+(b) applied (estimated by adding up the
  separate variants), the rows would sit at +2.4, +1.2,
  +5.8, +1.2, +1.1 and +1.1 Ggas of wasm over Oct 17. Per the bisect that is T4 revision (+4.3 on
  remove), A2 generation stamps (+0.7 to +1.8) and the rest of round 3 (≤ 1). The 12-byte header
  adds +0.5 Ggas of bytes on `Map` remove. This change does not touch them.
- **Offset by (c):** walking the slots in place removes a cost that predates Oct 17 (3.0-6.8
  Ggas per row). It more than covers the later commits, which is why op-only ends below Oct 17.
  Against an Oct 17 build that also had (c) (an estimate, not built), `Map` remove would still be
  about 6 Ggas behind, all of it from T4 and A2.

Not measured: h8, the `ext` rows (`try_insert`, `remove_step`, page) and the 1k/100k sizes.
They run the same flush and read path, so they should move the same way per slot and per read.
The `ext` (188,950 B) and h8 (163,341 B) builds compile.

Reproduce (from `proto-gas/sim` on `proto/storage-v2-sim-archive`, after building the wasm into a
directory):
`CARGO_TARGET_DIR=<own target dir> GAS_RES_DIR=<dir with v2_h16.wasm> cargo +1.95.0 test --test storage_v2_gas perf_rows -- --include-ignored --nocapture`
(`GAS_SIZES` overrides the default 10k).

**After merge (63d62e86 + perf).** The same `perf_rows` run on the merge commit `40124811`
(`proto/storage-v2` at `63d62e86`, verification round 2, merged with this branch), rebuilt with
`proto-gas/build.sh`, sim from a private `CARGO_TARGET_DIR`. Raw output:
`proto-gas/results/gas_merge_20261018.txt`.

| row (10k keys) | after fix (perf branch) | after merge | op-only after fix / merge (Ggas) | WASM_INSTRUCTION after fix / merge (Ggas) |
|---|---:|---:|---:|---:|
| `Map` insert new | 1.796 | 1.796 | 729.8 / 730.2 | 93.5 / 93.9 |
| `Map` update existing | 1.422 | 1.422 | 355.6 / 356.0 | 68.2 / 68.6 |
| `Map` remove non-last | 2.150 | 2.150 | 1083.8 / 1084.2 | 116.1 / 116.4 |
| `LookupMap` insert new | 1.315 | 1.315 | 248.7 / 249.1 | 57.7 / 58.1 |
| `LookupMap` update | 1.323 | 1.324 | 257.0 / 257.4 | 58.0 / 58.4 |
| `LookupMap` remove | 1.377 | 1.377 | 310.9 / 311.3 | 57.9 / 58.3 |

Every row is within 0.4 Ggas of the perf branch (target: 1 Ggas). Ops are unchanged
(3/4/0/0, 2/1/0/0, 5/3/3/0, 1/1/0/0, 1/1/0/0, 2/0/1/0), as are storage base, per-byte and
trie-node figures, so the +0.3-0.4 Ggas is wasm instructions only. cache.rs is byte-identical to the perf branch, so this is
attributed (by elimination, not a separate build) to the verification round's other production
changes: wire-typed record field keys in record.rs, plus collections.rs and root.rs edits. Wasm size: `v2_h16` 163,398 B (+37 B over 163,361 B),
SHA-256 `cc884e57…a6624b`. Contract loading unchanged at 178.0 Ggas (`ping` 1.066 Tgas).

The Oct 17 run follows, unchanged.


Oct 17, 2026. Branch `proto/storage-v2-gas` (from `proto/storage-v2` @ `7ca9d230`). Measured with
near-contract-sim (nearcore 2.14.0-rc.2, protocol 87), which runs nearcore's own `Runtime::apply`,
so these are the gas a node charges, to the unit. The simulator is deterministic: rerunning
gives identical numbers. The only spread is between *different keys* (trie placement). Each
insert/update/remove row covers 7 distinct keys, given as median (min-max).

## Verdicts

- **Entry hash width, 16 vs 8 bytes: doesn't matter at the Tgas level; keep 16.** The difference
  is 0.5-2 Ggas per operation, 0.05-0.1% of the receipt, which is less than the 5-9 Ggas spread
  between keys. Trie placement is identical at both widths. `accounts[alice].balances[usdt]` is
  59 bytes at h16 and 43 at h8.
- **Four-write `Map` insert.** Over `LookupMap`: +0.48 Tgas at 1k keys, +0.49 at 100k, +0.53 at
  100k keys with 100k other accounts. That is 2.9x the operation's own cost (0.25 vs 0.74 Tgas).
  Over v1 `IterableMap`: +0.28 / +0.29 / +0.30 Tgas. The doc's "index and header writes ≈ 0.2 Tgas"
  is true of those two writes alone, but the row should quote the full premium: ~0.5 Tgas over
  `LookupMap`, ~0.3 over v1. Removal: v2 `Map` costs ~0.2 Tgas *more* than v1 when the neighbor is
  small. What v2 buys is the bound (2.12 Tgas whatever the neighbor, against v1's 10.75 beside
  100 KiB), not a cheaper median.

## What was measured and how

- **Contract** `proto-gas/contract`: one source with the same methods over three builds:
  `v1` (`store::IterableMap` / `store::LookupMap` under the usual Borsh `STATE` root),
  `v2_h16` (`near_sdk::state`, 16-byte entry hash), `v2_h8` (same with the 8-byte hash, through the new
  near-sdk feature `unstable-state-hash8`, which sets `state::key::ENTRY_HASH_LEN = 8`). Keys are
  21-byte `AccountId`s (`hex(sha256("u{i}"))[..16].near`), values `u128`. Arguments are JSON. All
  three builds use the same release profile and are optimized with wasm-opt by cargo-near.
- **Harness** `proto-gas/sim/tests/storage_v2_gas.rs` (on `proto/storage-v2-sim-archive`): every
  scenario gets a fresh `Sim` with one contract and one populated collection. The contract seeds 8 keys itself. The harness checks
  every stored key and value byte for byte against its own layout model, then writes the other
  n-8 keys as state records in that layout. For v2 `Map` that covers entries, index, slots and
  header; for v1 it covers `ValueAndIndex` values, key slots and the length inside `STATE`. After
  that it checks `map_len` and the last key through the contract. `layouts_match_contract_writes`
  is the fast, non-ignored version of that check.
- **Breakdown columns** (Ggas, read from the receipt's gas profile): *storage base* = `STORAGE_*_BASE`;
  *per-byte* = the other `STORAGE_*` entries (key, value, evicted and returned bytes);
  *trie nodes* = `TOUCHING_TRIE_NODE` + `READ_CACHED_TRIE_NODE` (node count in brackets);
  *sha256* = `SHA256_*`; *wasm* = `WASM_INSTRUCTION`; *load* = `CONTRACT_LOADING_*`; *other* =
  everything else: the receipt's prepaid exec fees, register, memory, input and return costs.
  `r/w/d/h` = storage_read / write / remove / has_key calls. The *Tgas* column is the whole
  contract receipt (refunds excluded), so it includes about 1.04-1.06 Tgas that any function call
  costs (`ping` below). Subtract that to get the cost of the operation alone.
- **Trie.** "keys" = entries in the measured collection. Main tables: only `bench.near` holds
  contract data. The "+100k" table adds 100,000 other accounts' contract data (a `STATE` entry and
  one map entry each), which deepens the path to every key, as on a real chain.

### Reproduce

The `cd sim` steps need a checkout of `proto/storage-v2-sim-archive`.

```sh
cd proto-gas && ./build.sh                       # -> proto-gas/res/{v1,v2_h16,v2_h8,v2_h16_ext}.wasm (cargo-near, toolchain 1.95.0)
cd sim
export CARGO_TARGET_DIR=/Users/ricky/Developer/near-contract-sim-scenario/target   # reuse the nearcore build
cargo +1.95.0 test --test storage_v2_gas                                            # layout check, ~5 s
cargo +1.95.0 test --test storage_v2_gas gas_report -- --include-ignored --nocapture                   # 1k/10k/100k, ~50 s
GAS_SIZES=100000 GAS_OTHERS=100000 cargo +1.95.0 test --test storage_v2_gas gas_report -- --include-ignored --nocapture   # ~3 min
```

The raw output is in `proto-gas/results/gas_report.txt` and `gas_report_others100k.txt`. Key-length
check under both widths:
`NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features unstable,unit-testing --lib state::key`
(and the same with `--features unstable-state-hash8,unit-testing`).

## 1. Insert new / update existing: `Map` vs `LookupMap`, v2 vs v1 (Tgas, whole receipt)

| collection | 1k new | 1k update | 10k new | 100k new | 100k update | 100k + 100k accts new | 100k + 100k accts update |
|---|---:|---:|---:|---:|---:|---:|---:|
| v2 `Map` (h16) | 1.789 | 1.417 | 1.794 | 1.801 | 1.422 | 1.855 | 1.435 |
| v2 `LookupMap` (h16) | 1.309 | 1.318 | 1.312 | 1.314 | 1.322 | 1.328 | 1.336 |
| v1 `store::IterableMap` | 1.506 | 1.425 | 1.513 | 1.515 | 1.432 | 1.556 | 1.459 |
| v1 `store::LookupMap` | 1.395 | 1.413 | 1.400 | 1.402 | 1.422 | 1.429 | 1.450 |

Breakdown of insert-new at 100k keys plus 100k accounts (Ggas):

| collection | Tgas | storage base | per-byte | trie nodes (n) | sha256 | wasm | load | other | r/w/d/h |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| v2 `Map` | 1.855 | 425.9 | 10.2 | 148.2 (65) | 15.4 | 96.4 | 172.9 | 986.3 | 3/4/0/0 |
| v2 `LookupMap` | 1.328 | 120.6 | 3.7 | 36.5 (16) | 5.1 | 59.7 | 172.9 | 929.2 | 1/1/0/0 |
| v1 `IterableMap` | 1.556 | 305.3 | 9.3 | 107.2 (47) | 5.2 | 33.9 | 148.6 | 946.8 | 2/3/0/0 |
| v1 `LookupMap` | 1.429 | 241.1 | 7.4 | 73.0 (32) | 0.0 | 29.6 | 148.6 | 929.7 | 2/2/0/0 |

The counts include the per-call root: v2 reads the root record on every call, and v1 reads and
rewrites `STATE` on every `&mut self` call. So v2 `Map` new key = root read + index read + header
read, then 4 writes, as the doc states. v1 `IterableMap` = `STATE` read + value read, then value,
key slot and `STATE` writes. v1 `LookupMap` costs 2 writes because of the `STATE` rewrite.

**Premium of the four-write `Map` on a new key:**

| trie | Map − LookupMap (v2) | Map (v2) − IterableMap (v1) | op-only ratio Map : LookupMap |
|---|---:|---:|---:|
| 1k keys | +0.480 Tgas | +0.283 Tgas | 0.73 : 0.25 Tgas (2.9x) |
| 100k keys | +0.487 Tgas | +0.286 Tgas | 0.74 : 0.25 Tgas (2.9x) |
| 100k keys + 100k accounts | +0.527 Tgas | +0.299 Tgas | 0.79 : 0.27 Tgas (2.9x) |

The premium against `LookupMap` at 100k+100k breaks down as: 2 extra reads 113 Ggas, 3 extra writes
193 Ggas base, extra trie nodes 112 Ggas (49 nodes), extra sha256 10 Ggas, wasm 37, other 57.
Updating an existing key costs about 0.1 Tgas more on `Map` than on `LookupMap`: the index read
plus decode. Trie-node gas grows slowly with size: v2 `Map` insert goes from 36 nodes at 1k to 41
at 100k, and to 65 once 100k other accounts hold data, about 16 per write. That matches
references/gas.md.

## 2. Remove a non-last entry beside a 1 B vs 100 KiB neighbor (Tgas)

Two-key map. Remove the first key (16-byte value); the last key, which moves into the gap, holds
1 B or 102,400 B.

| collection | neighbor 1 B | neighbor 100 KiB | storage-only (base+bytes+trie), 1 B / 100 KiB | op-only (minus `ping`), 1 B / 100 KiB |
|---|---:|---:|---:|---:|
| v1 `store::IterableMap` | 1.910 | 10.745 | 0.680 / 7.719 | 0.873 / 9.708 |
| v2 `Map` (h16) | 2.116 | 2.116 | 0.774 / 0.774 | 1.055 / 1.055 |
| v2 `Map` (h8) | 2.114 | 2.114 | 0.772 / 0.772 | 1.053 / 1.053 |

v2 is neighbor-independent to the unit: the receipts are identical. v1's 100 KiB case adds,
term by term over the 1 B case: per-byte storage 7.04 Tgas, which is 102,400 B × (5.6 Mgas read +
31.0 write + 32.1 evicted), i.e. about 0.57 read + 3.17 write + 3.28 evicted bytes; then 1.46 Tgas of
register and memory cost (copying the value in and out of wasm) and 0.34 Tgas of wasm
(decode/encode). A rough estimate that counts only read and write bytes and leaves out the
evicted-byte charge and the register cost comes out near half of the measured figure. With a small neighbor v2 removal costs about
0.2 Tgas more than v1 (one more remove, more trie nodes, decode). With 100k other accounts: v1
10.813, v2 2.198.

## 3. Entry hash width, 16 vs 8 bytes (same keys, same trie shape)

| op (100k keys + 100k accounts) | h16 | h8 | Δ | spread across 7 keys |
|---|---:|---:|---:|---:|
| `Map` insert new | 1.855 | 1.854 | 1.4 Ggas bytes + 0.2 wasm ≈ 1.6 Ggas | 9 Ggas |
| `Map` update | 1.435 | 1.434 | ≈ 0.8 Ggas | 5 Ggas |
| `Map` remove | 2.241 | 2.239 | ≈ 2.1 Ggas | 7 Ggas |
| `LookupMap` insert new | 1.328 | 1.327 | ≈ 0.5 Ggas | 5 Ggas |
| `balance_of` (2 hops) | 1.288 | 1.288 | 0.5 Ggas | n/a |

Trie-node counts are identical between widths at every size: the trie branches within the first
few hash bytes, which both widths share. The difference is per-byte storage only, about 0.06-0.1%
of the receipt, smaller than the spread between keys. Key length for
`accounts[alice].balances[usdt]`: **59 bytes (h16), 43 bytes (h8)**, i.e. 27 + 2×width; checked by
`nested_two_level_key_length_tracks_hash_width`.

## 4. Root read per call and one nested read (v2 h16, Tgas)

| call | Tgas | storage base | per-byte | sha256 | wasm | other | r/w/d/h |
|---|---:|---:|---:|---:|---:|---:|---|
| `ping` (no `self`) | 1.061 | 0 | 0 | 0 | 0.1 | 888.1 | 0/0/0/0 |
| `noop_view` (`&self`: root read + decode) | 1.150 | 56.4 | 0.1 | 0 | 20.4 | 899.8 | 1/0/0/0 |
| `noop_mut` (`&mut self`, nothing changed) | 1.176 | 56.4 | 0.1 | 0 | 43.5 | 902.9 | 1/0/0/0 |
| `balance_of` (`accounts.at(id).balances.get(token)`) | 1.288 | 112.7 | 2.1 | 9.6 | 44.5 | 946.4 | 2/0/0/0 |
| v1 `noop_view` / `noop_mut` | 1.120 / 1.212 | 56.4 / 120.6 | | | | | 1/0 , 1/1 |

- `noop_mut` vs `noop_view`: v2 spends ~23 Ggas of wasm encoding the root to compare at flush, even
  when nothing changed.
- Root read per call: **89 Ggas** (56.4 read, 20 wasm decode, 12 other). An unchanged `&mut self`
  call in v2 writes nothing; in v1 it rewrites `STATE` (+92 Ggas, +105 with 100k accounts).
- Nested read below the root: **138 Ggas** (one 56.4 read, 2.1 bytes, 9.6 sha256, 24 wasm, 46 other,
  mostly JSON arguments and the return value).
- sha256: `SHA256_BASE` = 2 × 4.541 Ggas, so one host sha256 per hop (near-digest uses the host on
  wasm), **4.8 Ggas per hop** including bytes.

## Against the doc (`docs/storage-v2-spec.md`, rev 66)

| doc says | measured | verdict |
|---|---|---|
| read ≈ 56, write ≈ 64, remove ≈ 53 Ggas base | 56.4 / 64.2 / 53.5 | holds |
| sha256 per hop ≈ 4.5 Ggas | 4.54 base, 4.8 with bytes | holds |
| one write touches 9 nodes empty / 19 at 100k, 20-43 Ggas | 8-10 per write alone in the contract, ~16 with 100k keys + 100k data accounts | consistent. The worlds differ: here every extra account holds data (gas.md: every tenth), and keys are hashed v2 placements rather than raw token-map keys under mainnet-like ids |
| 8 extra hash bytes "under 0.6 Ggas per write" | 0.56 per written key; a `Map` insert carries it on 4 keys + 1 read ≈ 1.6 Ggas per op | holds per write; per op ~1.5-2 Ggas, still negligible |
| `Map` insert: "index and header writes together cost about 0.2 Tgas, trie nodes included" | those two writes ≈ 0.17-0.2 Tgas, but the whole premium over `LookupMap` (slot write, two reads, extra sha256 and decode) is **0.48-0.53 Tgas**; over v1 `IterableMap` **0.28-0.30 Tgas** | literally right, but understates what the table row implies. State the full premium |
| v1 `IterableMap` removal beside small vs 100 KiB neighbor: 0.58 vs 5.78 Tgas (figures from the measurement brief; not in `docs/storage-v2-spec.md`, presumably in the live doc's appendix C, which this session could not read) | 1.91 vs 10.75 whole receipt; 0.87 vs 9.71 op-only; 0.68 vs 7.72 storage-only | **contradicted** under every framing; the ratio is 5.6x (receipt) to 11x (storage) |
| v2 removal independent of neighbor | identical receipts, 2.116 Tgas either way | holds |
| `balance_of` "one read below the root" | 2 reads per call (root + entry); root costs 89 Ggas | holds as worded (below the root) |

## Not measured, or limits

- The doc's appendix C as it stands in the Claude Doc was not read: the doc id wasn't available
  to this session. The comparison above uses the committed spec plus the 0.58/5.78 figures from
  the task brief.
- Not a mainnet-sized trie: at most 100k keys plus 100k other contract-data accounts. Mainnet has
  millions, so expect a few more nodes per write (each ~2.3 Ggas) on every row, larger for rows
  with more writes. The ratios above barely move.
- The other accounts are contract-data records only, with no account or access-key records. Only
  the contract-data column shapes the path, but it is not the full `trie_gas.rs` world.
- The 100 KiB neighbor test uses a 2-key map; in a big map, add that map size's trie nodes to both
  columns.
- No sandbox differential run. The simulator's CI checks gas equality against a sandbox.
- Prototype artifacts inflate v2 a little: `Map::insert` hashes the key 3 times (entry path,
  index path, again in `push_key`) where once would do (~10 Ggas). The v2 wasm is 23 KB larger
  than v1, about 24 Ggas more load per call. Neither affects the two frozen numbers.
- Under `unstable-state-hash8` only the key-length assertions were adapted (made width-aware);
  the rest of the prototype test suite was not run under that feature beyond `--lib state::`.
