# near_sdk::state (storage v2) prototype notes

For the author of "NEAR SDK storage v2" (Oct 16, 2026 draft). Branch `proto/storage-v2`.
Everything is behind `unstable`. `store` and `collections` are untouched.

How to run:

```text
cargo check -p near-sdk --features unstable,unit-testing     # plain `unstable` hits near-sdk's native compile_error! by design
NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features unstable --lib state::
NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features unstable --test state_v2_exchange
(cd proto-wasm-check && cargo check --target wasm32-unknown-unknown)
```

(`NEAR_SANDBOX_BIN_PATH` only stops the `near-workspaces` dev-dependency build script from
downloading a sandbox binary, which this environment does not allow. It does nothing else.)

Results: 52 new unit tests, 13 integration tests, the full near-sdk lib suite (560 = 508 pre-existing + the 52 new), and the
49 near-sdk-macros snapshot tests pass.

## 1. What was built and what is stubbed

**Library (`near-sdk/src/state/`, hand-written)**

| File | Contents |
| --- | --- |
| `mod.rs` | the old `state.rs` (`ContractState`) unchanged, plus the v2 submodules |
| `key.rs` | `Path` builder (root, LEB128 field, kind, `h16 ++ borsh(k)` entry, u32 slot); a sealed `StorageKey` with `MAX_LEN` for ints, bool, `()`, `String`/`Vec<u8>` (4+512), `AccountId` (4+64), `[u8; N]`, pairs, `Option`. The hash is `near-digest` sha256 (host on chain, RustCrypto off chain), now enabled by `unstable`. |
| `record.rs` | the tagged codec: canonical varints, tag 0 rejected, **strictly ascending tags** (my addition, see §4), and an unknown-field merge on encode |
| `value.rs` | `Value` (blanket impl for Borsh leaves, `INLINE`, `MAX_SUBPATH`, bind/unbind), `Record` (view GAT), `Removable`, `ViewField`/`Scalar`, `removable_leaf!` |
| `cache.rs` | private `Host` trait (read/write/remove). This is the only code that calls `env::storage_*`. It holds a thread-local per-call cache with boxed slots, Clean/Touched/Dirty/Deleted marks, compare-at-flush, and a flush in key order with the root excluded and written last. The `unsafe` invariant is written at the top of the file. |
| `collections.rs` | `Map`, `IterMap`, `Vec`, `Lazy`, `Set`, `IterSet`, `GroupedMap` (groups not iterable), `Page`, entry APIs, `at()` |
| `root.rs` | `Root`, `load_root` / `flush_root` (the wrapper hooks), `raw_root` (raw tagged fields, the input for a `migrate` hook), and the hidden helpers the derive expands to |
| `testing.rs` | counting mock host: an op log with key, value length and replaced length per call, a sha256 count, `seed`, `assert_view_clean`, `use_env_host` |

**Macros (`near-sdk-macros`, generated)**: `#[near(storage)]`, `#[near(storage, reserved(3))]`,
`#[key(n)]`, `#[key(n, required)]`, `#[near(contract_state(layout = v2))]`, optional
`root = 0xF1`. The macro emits:

- `impl Value`: record encode with the unknown-tag merge, decode, bind/unbind, and `MAX_SUBPATH`.
- A `{Name}View<'a>` plus `impl Record`.
- `impl Removable where for<'x> FieldTy: Removable, ...`. The HRTB makes the impl inapplicable instead of a hard error when a field is a `Map`.
- For a root: `impl Root`, `ContractState` overrides, and placeholder Borsh impls (see §3).

The method wrapper codegen is unchanged. Macro validation catches a missing `#[key]`,
duplicate tags, tag 0, reserved-tag reuse and generic structs (generics are not supported).

**Hand-expanded**: `Account` and `Ex` in `state/tests.rs` are the macro's output written out,
because `::near_sdk` paths don't resolve inside near-sdk itself. Integration tests use the real
macros.

**Test contracts**

- **`near-sdk/tests/state_v2_exchange.rs`**: the spec's `Exchange`, with `deposit`,
  `balance_of`, `balances` and `cancel` through `GroupedMap`. It is driven exactly as the
  wrapper drives it: `state_read()`, the method, `state_write()`. Most tests use the counting
  mock; one runs the whole flow on the mocked blockchain. The file also covers v1-leaf tests
  and reserved tags.
- **`proto-wasm-check/`**: a standalone crate with the same contract, checked for wasm32 so the
  real `extern "C"` wrappers are type-checked against a v2 root. It passes.

**Stubbed or not built**: `Migrating`, the `migrate(old: RawRecord)` hook (only `raw_root`
exists), the ABI storage section, `storage_delta`, `#[gas_bound]`, the `bounded_work` lint,
group iteration in `GroupedMap`, `OrderedMap`, and Kani/bolero/fuzz harnesses. `clear(n)` exists
as `IterMap::clear_some(n)`. The debug `Drop` assertion that the wrapper flushed is not built.

## 2. Claims checked

Counts are written (reads, writes, removes), measured from the counting host's receipt.
Every cost test puts a 100 KiB value next to the entry under test.

| Claim | Evidence | Verdict |
| --- | --- | --- |
| Worked-example keys: 33 / 59 / 59 / 39 / 35 bytes, segment structure | `key::tests::worked_example_lengths_and_structure` checks every segment byte | **holds** |
| Addressable: no parent read before a nested lookup | `balance_of_via_at_is_one_read_and_never_loads_the_record`: the `Account` record key never appears in the receipt | **holds** |
| `balance_of` is "one storage read" | Exchange `deposit_balance_of_and_pagination`: **2 reads**, because the wrapper reads the root record before every method | **broken as stated**. It holds only for reads below the root. Every method pays the root read and decodes every root scalar. |
| Two paths cannot share a key; hot-bridge shape | Structural test: field tags diverge at byte 1, before any caller byte. No Kani or property test was written. | **holds by construction, not model-checked** |
| Local: removal never touches a neighbor's value | `itermap_remove_cost_is_independent_of_neighbor_value`: (4,3,3) with byte-identical receipts beside a 1 B and a 100 KiB moved neighbor; max bytes per host call ≤ 8 | **holds for Map / IterMap / Set / GroupedMap**. **Broken for `Vec::swap_remove`**, which reads and rewrites the last element, 100 KiB in `vec_costs`. The table says so, but the guarantee's wording ("never another entry's value") does not exempt it. |
| No debt | `no_debt_counts_do_not_depend_on_history`: insert and remove counts are equal on a fresh map and after 2,000 mixed ops | **holds** (differential test, not exhaustive) |
| One write per key per call, key order, root last | `flush_is_key_ordered_with_root_last` (4 accounts, each key once, ascending, root last); `balance_updated_ten_times_is_one_write` gives (1,1,0) | **holds for v2 keys**. **Broken with v1 leaves**: v1 writes land after the root via `Drop`, and are lost if nested (§4). |
| Insert then remove of a key already seen absent costs nothing | `insert_then_remove_of_key_seen_absent...` gives (1,0,0); for a key not seen, (0,0,1) | **holds under the stated condition**. Without the earlier read, the call pays a remove. |
| View methods never write | Type-enforced (`&self` cannot reach `put`); `view_call_writes_nothing` | **holds** (by types, not by a mock panic) |
| Nothing happens in `Drop` | v2 has no `Drop` impls | **holds** |
| Record is written only if a scalar changed or something below it was written | `deposit_costs_and_parent_write_rule`: an existing account with only a child changed gives (3,1,0), so the record is NOT rewritten | **holds under the tighter reading**. The literal reading ("written if anything below it was written") costs one extra write per touched parent per call: (3,2,0). Say which one is meant. |
| "An Account with a nonce and a balance map is a 10-byte record" | `record::tests::account_record_is_ten_bytes` | **holds** |
| Missing tag reads as `Default` | `missing_tag_reads_as_default` | **holds only for `Default` types**. The spec's `owner: AccountId` has no `Default` and does not compile; I added `#[key(n, required)]`. |
| Unknown tags preserved; reserved tags dropped | `unknown_tags_survive_a_rewrite`, `unknown_root_tag_survives_rollback`, `reserved_tag_is_dropped...` | **holds for read-modify-write**. **Broken for blind overwrites**: `insert` over a key not read in this call drops them (`unknown_tags_on_overwrite_depend_on_whether_the_key_was_read`). |
| Tag 0 and non-canonical varints rejected | record tests plus stored-record tests that panic at decode | **holds** |
| A scalar whose Borsh length changes fails loudly | `scalar_length_change_fails_loudly` (u32 to u64) | **holds** |
| A scalar-to-collection swap under the same tag is silent | `scalar_to_collection_swap_is_silent`: the old bytes are carried forward as unknown and the collection starts empty | **holds (the silence is real)** |
| Linear iteration: a page with a cursor costs 2n + 2 | `itermap_pagination_is_linear`: cursor gives 2n + 2; no cursor gives 2n + 1; `keys` gives n + 1; `iter` gives 2n + 1 | **holds** |
| Cursor pages can skip a key after an interleaved removal | `page_skips_moved_key_after_interleaved_removal`: key 5 is never returned | **holds (as documented)** |
| Key length limit refused at build time | `MAX_SUBPATH` summed per type; `load_root` const-asserts it. A probe with 4 nested `Map<String, _>` levels fails with E0080 | **holds**, but as a post-monomorphization error: `cargo build`/`test` catch it and `cargo check` generally does not |
| Removing a record clears its collections | `removing_a_record_clears_its_collections`: (6,0,11) with 3 balances; the store keeps only the root | **holds**. The cost is 1 header read + 1 slot read per entry + 3 removes per entry + header remove + record read/remove; the header read is unstated. |
| `remove` on a non-Removable record does not compile | Probe: `Map<u32, Holder{Map}>::remove` gives E0277 "`Map<u32,u32>: Removable` is not satisfied" | **holds**, with a readable error |
| The spec's example compiles once the macros exist | Exchange compiles natively and for wasm32 with the real wrapper | **holds with 3 edits** (§3) |

**Cost table, row by row** (measured vs the table):

| Op | Table | Measured | Note |
| --- | --- | --- | --- |
| Map get / insert / update / replace / remove / discard | 1r / 1w / 1w + old bytes / 1r 1w / 1r 1d / 1d | same | `replaced_len` checked |
| IterMap insert new | 2r 4w | 2r 4w | `entry().or_default()` is also 2r 4w: entry read in place of index read |
| IterMap update existing | 1w | **1r 1w** | `insert` cannot know the key exists without the index read; `get_mut` is 1r 1w. "1 write" is only true when the read already happened in this call. |
| IterMap remove, key not last | 4r 3w 3d | 4r 3w 3d | |
| IterMap remove, key is last | not stated | 3r 1w 3d | no moved key |
| IterMap remove, map empties | not stated | 3r 0w 4d | I remove the header at 0 (no debt); the table says header write |
| Vec push | 1r 2w | 1r 2w | |
| Vec pop | 1w 1d | **2r 1w 1d** | header read and slot read (to return the value) are missing from the table |
| Vec swap_remove | (pop) + 1r 1w = 1r 2w 1d | **3r 2w 1d** | same two reads missing |
| Lazy get / set / remove | 1r / 1w / 1d | same | key is `parent ++ tag`, no kind byte |
| GroupedMap remove, group empties | as IterMap + 1 header remove | 3r 0w 4d | |
| sha256 | not in the table | 1 per map hop per op | `balance_of` = 2; the one-call scenario = 6 (counted). Not measured here: by the protocol config, `sha256_base` is about 4.5 Ggas vs about 56 Ggas `storage_read_base`. |

**The "one call through the cache" scenario**: existing alice with {usdt, dai, eth}; deposit
usdt twice, bump the nonce, remove dai (not last). The measured cost is 7 reads, 5 writes,
3 removes and 6 sha256 (`one_call_through_the_cache`). It matches the formulas exactly:

- **Reads (7)**: root, account record, usdt entry, then the IterMap remove's dai entry, dai index, header and last slot.
- **Writes (5)**: usdt entry, account record (the nonce changed), the moved slot, eth's index, the header.
- **Removes (3)**: dai entry, dai index, last slot.

The doc's 4/2/1 diagram leaves out the removal's index and slot keys and the root read.

## 3. Usability friction

**What compiled verbatim**: the bodies of `deposit`, `balance_of`, `balances` and `cancel`,
character for character. `let acct = ...or_default();` works without `mut` because
`or_default()` returns `&mut Account`.

**Three edits the spec's example needed**:

1. `#[key(1)] owner: AccountId` gives E0277 `AccountId: Default`. Fix: `#[key(1, required)]`, so
   a missing tag is a decode error. Every non-`Default` scalar needs this (`AccountId`, most
   enums, v1 collections).
2. `self.orders.group_mut(&market).remove(&order)` gives E0277 `Order: Removable`. Every plain
   Borsh value struct in a removable map needs `near_sdk::state::removable_leaf!(Order);`. The
   spec mentions the opt-in, but the example omits it, and in practice it applies to almost
   every value type.
3. The root needs `#[derive(PanicOnDefault)]`. The wrapper still calls
   `state_read().unwrap_or_default()`, and v2's `state_read` never returns `None`, but the
   `Default` bound remains.

**API divergences from the spec, and why**

- **`or_default()` returns `&'a mut Record`, not a generated `ViewMut`.** The record has to be read
  anyway to know whether it exists. Handing out the decoded record gives real field access
  (`acct.nonce += 1`) and needs no proxy type. Scalar changes are detected by re-encoding and
  comparing at flush, which costs CPU, not gas. I found no case where a separate `ViewMut` adds
  anything. An `at_mut` that skips the record read can't uphold the "record exists before its
  children" rule.
- **View scalars are `view.nonce.get() -> Option<&u64>`, not `view.nonce`.** Collections are
  fields typed as bound handles (via `ViewField`), so `at(&id).balances.get(..)` reads exactly as
  in the spec. A scalar can't be a plain field without reading the record eagerly.
- **The root is not a `Value`.** The wrapper's `ContractState::state_read`/`state_write` carry
  `where Self: Borsh*`. An override can't drop those bounds, so the macro emits placeholder
  Borsh impls (`unreachable!`). Those collide (E0119) with the blanket
  `impl<T: Borsh> Value for T`, so the root gets its own `Root` trait instead. Side effect: a v2
  root counts as a Borsh leaf and would type-check as a map value, then panic at runtime. The
  real fix is a bound-free load/store pair on `ContractState` called by the wrapper. That
  changes every macro snapshot, so I didn't do it.

**Borrow checker** (compile probes, not committed):

- **E0716**: `let bal = accounts.at(&id).balances.get(&t); use(bal)`. The view's handles are
  owned temporaries, so a reference read through them dies at the end of the statement. Chained
  use (`.copied()`) is fine, and binding the view first works. A fix would make view handles
  carry `'a` and return `&'a V`, which duplicates the read API per collection.
- **E0499**: a transfer, `let a = accounts.entry(from).or_default(); let b = accounts.entry(to).or_default();`.
  The two `&mut` records conflict, so the code has to be written as two sequential blocks.
  Gas is the same because of the cache. v1 has the same problem, but per-account nested records
  make it the common case.
- **E0502**: holding a `get` reference across an `insert` is rejected. This is the cache's
  aliasing invariant doing its job.

**Smaller friction**

- Handles are deliberately not `Clone`, so a record with collections can't derive `Clone`,
  can't be returned from a view as JSON, and can't be paged (`page` needs `V: Clone`).
- `use near_sdk::state::*` shadows `std::vec::Vec` with `state::Vec`.
- `IterMap` needs `K: Clone`.
- Error quality: missing `#[key]`, duplicate tags, the non-Removable `remove` and `Order: Removable` all give pointed messages. The post-mono guards give E0080 with the assert text but point into core's `panic.rs`.
- Recursive schemas (a tree of `Node { children: IterMap<u32, Node> }`) give E0275 (trait-solver
  overflow on the HRTB `Removable` bound). Even without that bound, `MAX_SUBPATH` would be a
  const cycle, so trees can't be expressed.

## 4. Edge cases and gaps the doc does not cover

**Soundness of reference-returning `get`.** `get` returns `&V` into the cache, which needs
`unsafe`. The invariant is written in `cache.rs`:

- slots and values are separate allocations, never freed within a call;
- a value is dropped or overwritten only through `&mut` on its owning handle;
- **exactly one handle can produce `&mut` to a key.**

The last rule is not type-enforced. Ways to break it:

- **Two roots alive at once**, which tests can do by calling `state_read()` twice.
- **`Map::__bound_at` / `Path` are public.** They have to be, for the derive, so any code can
  build a second handle to any path. "Only a `Path` can make a `Key`" holds, but `Path` is freely
  constructible from its builders.
- **The doc should state the single-root rule** and the macro should make roots hard to construct
  twice. The spec's "never freed within a call" is necessary but not sufficient.

**Binding hazards** (all have tests):

- `insert` of a fresh record over an existing key keeps the old record's collections. The new
  empty handle binds to the same path and sees the old children.
- `remove` + `insert` under another key: `remove` clears the children, so the moved record
  arrives empty. No error, the data is gone. The doc should say moving a record is
  remove + rebuild.
- `remove` returns the record with its collections **unbound** (touching them panics). Without
  that, writing through the returned value would recreate orphan keys under the removed path. So
  a contract must read a record's balances (e.g. to refund them) *before* `remove`.
- `mem::take` / `mem::replace` on a `&mut Record` moves a bound handle out and leaves an unbound
  one in the cache. Inserting the taken record elsewhere panics ("already bound"), and the
  original key's collections now panic as unbound.
- `Vec<T>` where `T` owns collections: `swap_remove` would move the record bytes but leave its
  children at the old slot path. I made it a compile error (`T::MAX_SUBPATH == 0`). The doc
  should forbid moving collection-owning records.
- `Map<K, Map<..>>` (a collection as a map value) has no existence marker. It's a compile error
  pointing at `GroupedMap`.

**Encoding corner cases**

- **Tag order.** The spec says "every record has exactly one encoding", but the grammar allows
  fields in any order. I require strictly ascending tags and reject duplicates. Freeze that in
  the format spec.
- **Unknown tags vs overwrite** (the history-dependent case in §2). Fix: make `insert` of a
  record read the old bytes, or define "a blind overwrite drops unknown tags".
- **Unknown tags vs moving a record.** Unknown fields are merged from the bytes stored *at the
  same key*, so they don't follow a moved record. Keeping them in the cache rather than as a
  hidden struct field keeps struct literals clean; a hidden field would break
  `Account { nonce, balances }` syntax.
- **A stored scalar under a tag that is now a collection** is carried forward as unknown
  forever. It never decodes and never drops unless the tag is `reserved`.
- **`Set` values are zero-length writes.** Not tested on the real runtime.

**Root lifecycle**

- The first mutating call on a never-written root writes the root record (absent -> created with
  defaults), even if no root scalar changed.
- The init body runs before the wrapper can bind root collections. `Self { accounts: Default::default(), .. }`
  is fine, but touching `self.accounts` inside `#[init]` panics "not bound". This is a real
  restriction for contracts that seed state in `new()`.
- **`state_exists()`** (the init-twice guard) calls `env::storage_has_key` directly, bypassing
  the `Host` and the cache. It's correct on chain but invisible to the counting host. It's one
  more host path the "only the cache reaches the host" claim has to cover.
- Every method pays one root read plus decoding every root scalar. Large root scalars belong
  in `Lazy`, so the doc's advice should be stronger than "for large fields".

**v1 leaf fields (Coexistence step 1)**

- **At the root**: the v1 `LookupMap` flushes in `Drop` after `state_write`. Its writes land
  *after* the v2 root, so "root last" doesn't hold (`v1_leaf_at_root_flushes_on_drop_after_the_v2_flush`).
- **Nested in a record that lives in the v2 cache** (e.g. `Map<u32, User { notes: LookupMap }>`):
  the cache never drops values within a call, so the v1 write is **lost silently**
  (`v1_leaf_nested_in_a_cached_record_never_flushes`). On wasm, destructors of leaked memory
  never run either.
- **What the doc should say instead**: v1 leaves are allowed only at the root (or the cache
  must call an explicit `flush` on them during the v2 flush). They also need `required`, since
  v1 collections have no `Default`.

**Key length**

- `MAX_LEN` is enforced at runtime on every `entry()` as well as summed at build time. A
  caller-supplied `String` key over 512 bytes now panics where v1 accepted it. That's a new,
  caller-triggerable failure mode; it's only the caller's own transaction, but worth stating.
- **`StorageKey` "sealed, but emitted by the SDK's Borsh derive"**: a derive in the user's crate
  can only implement a trait the user's crate can name, so the seal is a convention. A
  `StorageKey` derive also can't tell whether the type's `BorshSerialize` was derived or
  hand-written, so "a hand-written Borsh impl is not accepted" isn't enforceable unless the
  `StorageKey` derive generates the encoding itself.

**Cache memory and CPU**

- Iterating n `IterMap` entries holds 2n + 1 slots for the rest of the call (`cache_holds_every_touched_key_for_the_call`).
- Overwrites reuse the slot, but nothing is freed until the call ends.
- Every touched record is re-encoded at flush to detect changes.

**Small semantics the doc leaves open**

- **`page` with a removed cursor.** The spec says "returns an error", but the example signature
  returns `Page` directly. I panic. Decide on `Result<Page, _>` vs panic.
- **`contains_key` reads and decodes the value.** A 100 KiB value is read just to test
  existence. `storage_has_key` would be cheaper, but the cache would need a "present, bytes
  unknown" state. Worth a row in the table.
- **Flush order puts parents before children**, because a record key is a prefix of its
  children's. That's harmless since the receipt is atomic; it's a determinism convention, not a
  consistency one.

## 5. Suggested changes to the design doc

- **`balance_of`**: change "one storage read" to "one read below the root (two with the wrapper's root read)". State that every method pays the root read.
- **Cost table**: add the missing reads (Vec `pop` and `swap_remove`: header + returned slot; `Removable` clear: header read). Add rows for IterMap remove-last, IterMap empties, and the sha256 count per map hop. Qualify "update existing: 1 write" as "after a read in the same call; `insert` alone is 1 read + 1 write".
- **Local**: say "never another entry's value, except `Vec::swap_remove`, which moves the last element".
- **Parent-write rule**: say which reading is meant. I'd recommend "an absent record is created; an existing record is rewritten only if its scalars changed".
- **Record format**: require strictly ascending tags, and define what a blind overwrite does to unknown tags.
- **Missing tag -> `Default`**: add `required` (or make the scalar `Option<T>`) for non-`Default` types, and fix the example's `owner`.
- **`removable_leaf`**: show the opt-in in the example (`Order`), or make `#[near(serializers=[borsh])]` value types Removable by default and require an opt-*out* for types that hold storage (v1 collections).
- **Moving records**: spell out that moving between keys is remove + rebuild, that `remove` returns unbound collections, and that `Vec` can't hold collection-owning records.
- **v1 leaves**: allow them only at the root, or have the v2 flush flush them; nested v1 leaves lose writes. Correct "keeps v1's write-on-drop" accordingly, and note that their writes land after the root.
- **Wrapper**: add a bound-free load/store pair to `ContractState` so v2 roots needn't fake Borsh. Drop the `Default` bound for v2. Route `state_exists` through the cache/host.
- **Single root**: state the single-root aliasing rule as part of guarantee 4's mechanism. Also state that `cargo check` misses the post-mono guards (key length, map-of-maps, `Vec` move).
- **`page` errors**: decide `Result` vs panic for a removed cursor, and make the example's signature match.
- **Recursive schemas**: state that trees are out of scope (the key length bound needs a finite schema).
- **`StorageKey` sealing**: reword it as "derive-only by convention", or have the `StorageKey` derive generate the encoding.
- **Init**: note that `#[init]` can't touch root collections, or have the macro pre-bind root fields in a constructor helper.

## 6. What I did not get to

- **`Migrating<V1, V2>`, the `migrate(old: RawRecord)` hook** (only `raw_root()` exists, untested), **and the ABI storage section.** Under `__abi-generate`, `Page` would also need `JsonSchema`.
- **`storage_delta()`, the NEP-145 interaction, `#[gas_bound]`, and the `bounded_work` lint.**
- **Kani, bolero, fuzz and cargo-mutants harnesses.** The prefix-freeness and round-trip claims are covered only by structural unit tests.
- **Group iteration in `GroupedMap`, and `OrderedMap`.**
- **The real runtime.** Gas figures are counts from the mock, not near-contract-sim Tgas. Zero-length values (`Set`) and the trie-node term are untested on the runtime.
- **The debug `Drop` assertion on the root, and generic records in the derive.**
