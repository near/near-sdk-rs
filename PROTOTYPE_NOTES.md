# near_sdk::state (storage v2) prototype notes

Written for the author of "NEAR SDK storage v2" (spec: Oct 16, 2026 draft).
Branch `proto/storage-v2`. Status: INTERIM (core + tests done, macros in progress).

## Interim findings (to be folded into the sections below)

- Core built: key builder, record codec, per-call cache behind a private `Host`, counting
  mock, Map/IterMap/Vec/Lazy/Set/IterSet/GroupedMap, views, Removable. 51 unit tests pass.
- Worked-example key lengths (33/59/59/39/35) hold.
- IterMap removal: (4 reads, 3 writes, 3 removes) beside a 1 B and a 100 KiB neighbor, byte-identical
  receipts. Removing the last key is (3,1,3); emptying removes the header.
- Table omissions: Vec::pop needs header read + slot read; IterMap "update existing = 1 write"
  only if the caller already read the key (insert pays the index read); Map::clear/Removable
  needs a header read; sha256 calls are not in the table (6 in the one-call scenario).
- Parent-write rule: with compare-at-flush, a child write does not rewrite an unchanged parent.
- `insert` over an existing record keeps old children; `remove` + `insert` elsewhere drops them;
  `mem::take` on `&mut Record` breaks binding.
- First call on a fresh contract writes the root (absent -> created with defaults).
- `cargo check -p near-sdk --features unstable` fails natively by design (compile_error); use
  `unstable,unit-testing`. `cargo test` needs `NEAR_SANDBOX_BIN_PATH=/usr/bin/true` in this sandbox.
