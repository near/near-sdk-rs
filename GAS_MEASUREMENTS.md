# Storage v2 prototype: gas measured on the real runtime

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
- **Harness** `proto-gas/sim/tests/storage_v2_gas.rs`: every scenario gets a fresh `Sim` with one
  contract and one populated collection. The contract seeds 8 keys itself. The harness checks
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

```sh
cd proto-gas && ./build.sh                       # -> proto-gas/res/{v1,v2_h16,v2_h8}.wasm (cargo-near, toolchain 1.95.0)
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
