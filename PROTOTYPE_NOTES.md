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

## Revision 1 results

Applied `/tmp/storage-v2-proto/revision-1.md` on `proto/storage-v2` (commits `3ff04047`, `037d8100`, `3bd455e3`, plus this notes commit).

### Success check

**Passes.** `proto-wasm-check/src/lib.rs` holds the spec's `Exchange` and `Account` verbatim. The only edit is `#[key(1, required)]` on `owner`. There are no derives on either struct, no `PanicOnDefault`, no `removable_leaf!`, and no imports beyond the spec's own. The types the spec leaves undefined (`TokenId`, `MarketId`, `OrderId`, `Order`, `Metadata`) are declared above it as plain Borsh/JSON types.

- `cargo check --target wasm32-unknown-unknown`: ok.
- `cargo check --features near-sdk/unit-testing` (native): ok. Plain native `cargo check` hits near-sdk's existing non-wasm `compile_error!`, which is not new.
- `cargo build --release --target wasm32-unknown-unknown`: ok, 270,502 bytes.

**Correction to round 1.** §2 said the example "holds with 3 edits". It was 4. I silently added `#[derive(Default)]` to `Account` in round 1, and the spec's `Account` has no derives. Revision 1 removes the need for it, because the derive now emits `Default` for any record without a `required` field.

### Per item

| # | Status | Reason |
|---|---|---|
| 1 | **changed** | The derive can't tell a scalar field from a collection without type information, so the view can't *omit* scalars. Every field exists in the view, typed through `ViewField::Out`. Records give a nested view, collections give `At<'a, C>`, and every leaf gives `ScalarNotInView`, a ZST with no methods. `at(&id).nonce.get()` fails with "no method named `get` found for struct `ScalarNotInView`". The user-visible rule ("at walks, get reads") holds. The field is still there in autocomplete. |
| 2 | applied | No `ViewMut`. `entry().or_default()` and `get_mut` return `&mut V` with bound handles. |
| 3 | applied | `At<'a, C>` read methods return `&'a V`. `let bal = self.accounts.at(&id).balances.get(&t);` compiles (unit test). Does not extend to `group_mut(&m).get_mut(&k)`, see friction. |
| 4 | confirmed | The sequential two-`entry` transfer compiles. It costs (5 reads, 2 writes, 0 deletes): root, two records, two balances read; two balance writes. The records are not rewritten. Test `sequential_transfer_pattern`. |
| 5 | applied | `FieldDefault` with `#[diagnostic::on_unimplemented]`. The error reads "storage field of type `AccountId` has no `Default`, so an old record without this tag could not be decoded", with the label "write `#[key(n, required)]` on this field, or make it `Option<AccountId>`" and a note on the evolution rule. It is emitted **twice** per field (once from the decode path, once from the generated `Default`). Test `required_field_added_later_fails_old_records` shows a v1 record failing with `Missing { tag: 2 }`. |
| 6 | applied, **premise wrong** | `removable_leaf!` is gone, Borsh leaves get a blanket `Removable`, and records get the derived HRTB-conditional impl. The "no leaf can orphan anything" justification does not hold; see below. |
| 7 | already done | Strictly ascending tags were enforced in round 1. |
| 8 | applied | Blind `insert` replaces the record whole and drops unknown tags, even if the record was read earlier in the call. `get_mut` and `entry` keep them. Test `blind_insert_drops_unknown_tags_get_mut_keeps_them`. |
| 9 | applied | The cache frees every slot after the flush, outside the `RefCell` borrow. A nested v1 field's write-on-drop now lands, after the root (test `v1_leaf_nested_in_a_cached_record_writes_late_not_lost`; round 1 lost it). The derive rejects `LookupMap`/`UnorderedMap`/`IterableMap`/`Vector`/`LookupSet`/... by last path segment in non-root records. A type alias evades it; the test suite uses one on purpose. `store::Lazy` vs `state::Lazy` can't be told apart by name, so `Lazy` is not in the list. |
| 10 | **changed** | Done trait-side, not wrapper-side. `#[near]` on an impl block can't see whether `Self` is a v2 root, so routing v2 through different wrapper tokens would change v1 output too. Instead, `ContractState::state_read`/`state_write` now bound on new `StateRead`/`StateWrite` traits. Their Borsh blankets reproduce the old bodies; the v2 derive implements them with `load_root`/`flush_root`. The wrapper and all macro snapshots are byte-identical to master (`git diff master` on the snapshot dir and wrapper codegen is empty; 49 macro tests pass). "Update macro snapshots" in the revision conflicts with the round-2 brief; I followed the brief. |
| 11 | doc | Unchanged. The eager root read is still every method's first read. |
| 12 | applied | `Root::bind(self) -> Self`, provided method, idempotent. It needs `use near_sdk::state::Root` in scope, so the doc's one line is really two. I didn't find a call-free way: a struct literal runs no code, and an unbound handle doesn't know its own tag. Without `bind`, touching a collection in `#[init]` panics with a hint to call it. |
| 13 | partial | `state_exists` goes through the host (`root_exists`, 1 `has_key` in the counting mock). `state_read` can't be hidden: it is a provided method of a public trait, and the wrapper calls it from the user's crate. |
| 14 | holds | Already the prototype's behaviour. Covered by the cost tests. |
| 15 | applied, extended | `insert` of a record with `MAX_SUBPATH > 0` costs 1 `has_key` (or a cache hit) + 1 write, and panics "state: record exists; use get_mut, or remove it first". It compiles for every `V`. Leaves stay a blind 1-write. I applied the same rule to `Lazy::set`. `Map::replace` now requires `V: Removable` and clears the old children. The revision doesn't mention `replace`, and its returned record otherwise has the same resurrection hazard. |
| 16 | applied | `remove` clears children and returns a detached record. Its handles panic "collection detached by remove; read it before removing". Re-inserting a detached record panics ("build a new record instead"). Inserting a record still bound at another key panics with a `mem::take`/`mem::replace` message; detection costs one enum check. |
| 17 | applied | `contains_key` uses `storage_has_key` when the key is uncached: 1 host call, 0 bytes. New `Orig::PresentUnread` cache state, so a later `get` still reads. Test `contains_key_uses_has_key`. |
| 18 | added `keys_page` | `keys(after, n)` + `get` works but makes the client compute the next cursor. I added `keys_page(after, limit) -> KeyPage { keys, next }`. Building a JSON response from `&V` records is still hand-written per contract. Awkward, not blocking. |
| 19 | applied | The removed-cursor panic reads "page cursor key no longer exists (removed since the last page); restart from a known key". |
| 20, 21 | doc | The `contains_key` row is measured (item 17). The other rows were measured in round 1. |
| 22 | **not implemented** | No runtime key-length check in `entry()`, and the `StorageKey` derive was not changed. |
| 23, 24 | doc | No code. |

### Where I think the revision is wrong

- **Item 6's premise.** "The only Borsh types that own storage are v1 collections, and they are root-only (item 9), so no leaf can orphan anything." Root-only is a doc rule plus a best-effort name check, not a type-level fact.
  - A type alias, or any user Borsh struct that *wraps* a v1 collection, passes the check, gets the blanket `Removable`, and makes the enclosing record `Removable`.
  - Test `legacy::removing_a_record_with_a_nested_v1_field_orphans_its_entries` shows `users.remove(&2)` compiling and leaving the v1 entry `n2‖7` in storage with nothing pointing at it.
  - Round 1's opt-in had the opposite failure mode: a missing impl was a compile error, not an orphan.
  - If the blanket stays, item 9's rejection is the only guard, and the doc should say so rather than "no leaf can orphan anything".
- **Item 10's "no `Default` bound".** The wrapper is unchanged (the snapshot constraint requires it), and non-init methods still call `state_read().unwrap_or_default()`. The v2 root therefore still needs `Default`. The derive emits one (an empty-record decode, so `required` fields panic) unless the user already derives `Default`, `PanicOnDefault` or `SmartDefault`. The bound is hidden, not removed. Removing it for real means a v2-specific wrapper path, and that needs either a marker the impl-block macro can see or changed v1 snapshots.
- **Item 10 edits a public v1 trait.** `ContractState` exists on master. Adding `where Self: StateRead` to its provided methods breaks any downstream override written as `fn state_read() -> Option<Self> where Self: BorshDeserialize`, which becomes E0276 ("stricter requirements"). I know of none in practice, but it is a semver-visible change.
- **Item 15's "for leaf values nothing changes"** is right, but the dividing line is `MAX_SUBPATH > 0`, not "owns collections" as a user reads it. A record with only scalars is a leaf under this rule and keeps blind-overwrite semantics, including dropping unknown tags (item 8). That is consistent, but the doc should state the rule by what the type contains, not by "record vs leaf".

### New friction and edge cases from this revision

- **`ScalarNotInView` fields** appear in autocomplete and only fail at the method call.
- **`group_mut(&m).get_mut(&k)` bound to a `let` is still E0716.** Item 3's lifetime fix covers `At` reads, not the `GroupMut` temporary. The one-statement form `group_mut(&m).remove(&k)` is fine.
- **Duplicate item-5 diagnostics.** The same message is emitted twice per offending field.
- **Drop-after-flush soundness caveat.** Freeing slots at the flush invalidates any `&V` obtained before a *manual* `state_write()` in the same call. The wrapper never does this. Hand-written code that calls `state_write()` mid-method and then keeps using an old reference is UB in the prototype. A debug-mode generation counter would catch it; not done.
- **Borsh-deriving a v2 root** now fails with E0119 "conflicting implementations of `StateRead`" (plus the Borsh errors on the collections). Correct, but the message names an internal trait.
- **`required` on a collection field** is rejected at decode time (`RecordError::Leaf`), not at compile time.
- **Insert-new adds a read to the init path.** Seeding a new record in `#[init]` via `insert` costs a `has_key`; via `entry().or_default()`, a read. Neither existed in round 1's blind insert.
- **The `bind` import.** `Exchange { .. }.bind()` without `use near_sdk::state::Root` gives "no method named `bind`" with a trait-import hint. Fine, but it is the second line item 12 promised would be one.

### Test counts

- `near-sdk` lib `state::` unit tests: **56 passed**.
- `near-sdk/tests/state_v2_exchange.rs`: **17 passed**, including 3 `should_panic`.
- `near-sdk` full lib (`unstable,abi,unit-testing`): **565 passed**, 1 ignored.
- `near-sdk` doc tests: **343 passed**, 6 ignored.
- `near-sdk-macros`: **49 + 1 passed** (2 + 2 ignored). Snapshots unchanged against master.
- `proto-wasm-check`: wasm32 check, native check and wasm32 release build all ok.

## Revision 2 results

Applied `/tmp/storage-v2-proto/revision-2.md` in commit `8b37bb4f`.

### Success check

**Still passes.** `proto-wasm-check/src/lib.rs` is unchanged this round, with `required` on `owner` as its only edit. wasm32 check, native check (`--features near-sdk/unit-testing`) and wasm32 release build (270,717 bytes, +215 over round 2) all ok.

### Correction to round 2

The round-2 table marked revision-1 item 22 "not implemented: no runtime key-length check in `entry()`". That was wrong. `Path::entry` has checked `borsh(k).len() <= K::MAX_LEN` since round 1, and every `entry`/`get`/`insert`/`remove`/`contains_key` goes through it. A key-level unit test already existed. What was missing was a test through a collection, which this round adds. The `StorageKey`-derive half of item 22 is still not done.

### Per item

| # | Status | Reason |
|---|---|---|
| 1 | **changed** | Not "methods on the group handle return `'a` references". On a `&mut self` handle that is unsound: two `get_mut(&k)` calls on the same key would give two live `&'a mut V`. Making the methods consume the handle instead breaks a named handle after one call. Instead, `GroupMut<'a, K, V>` is now a type alias for `&'a mut IterMap<K, V>`. `group_mut` moves a fresh `IterMap` handle into a per-call arena in the cache, freed with the slots at the flush (same invariant 4 as records), and returns `&'a mut` to it. Every `IterMap` method then borrows from the `GroupedMap`, not a temporary. Both `let o = self.orders.group_mut(&m).get_mut(&k);` (a `fill` method in the `#[near]` impl) and a named `let g = group_mut(&7)` used for `get_mut`, `get_mut`, `get`, `insert` compile. Ordinary borrowck still prevents aliasing. Test `group_mut_references_outlive_the_temporary`; `fill` costs 1 root read + 1 order read + 1 write, the same as a plain `IterMap`. |
| 2 | applied | The derive emits `impl Exchange { pub fn bind(self) -> Self }` forwarding to `Root::bind`, outside any `#[near]` block, so it is not exported. The inherent fn wins over the trait method, so having both in scope is not ambiguous. The exchange test dropped its `use ...::Root` and `new_seeded` still compiles. |
| 3 | already held; test added | See the correction above. New test `oversized_key_panics_naming_the_limit` inserts a 600-byte `String` token into `balances` and gets "storage key is 604 bytes, over its declared MAX_LEN of 516". **Friction:** the limit is in borsh bytes, so a `String` key documented as 512 bytes reports 516 (the 4-byte length prefix). The message is accurate but will read as off-by-four to anyone thinking in characters. |
| 4 | applied | The generated record `Default` no longer spells the fields itself. Each record gets a private `__decode_fields(fields, Option<&Path>)` holding the one spanned `field::<T>` call per field. `decode_value` calls it with `Some(path)`; `Default` calls it with no fields and `None`, which leaves collections unbound. `__private::field` / `field_required` now take `Option<&Path>`, and `field_default` is gone. A record with `#[key(1)] who: AccountId` now produces exactly one E0277, pointing at the field type (checked with a scratch probe). |
| 5 | **rejected, documented** | A plain `&V` has no hook on use, so nothing can fail loudly short of returning guard types from every getter, which is the API change revision 1 decided against. The options I looked at don't fail *loudly*: keeping freed allocations alive under `unit-testing`, or poisoning their bytes after `drop_value`. The first still reads a dropped value; the second is UB that may or may not crash. Miri catches it reliably. `ContractState::state_write` now documents the rule: drop every reference before a manual `state_write()`, and re-read afterwards. |
| 6 | applied | Renamed to `UseGetForScalars`, `#[doc(hidden)]`. `at(&id).nonce.get()` fails with "no method named `get` found for struct `near_sdk::state::UseGetForScalars`" (probe). It is still a field in autocomplete, but now its type says what to do. |

### New friction and edge cases

- **The arena allocates per `group_mut` call.** A method that calls `group_mut` in a loop over N groups holds N small handle allocations (a `Path` each) until the flush. That is bounded by the call and similar in size to the records the same loop would cache, but it is new memory that a view-only loop over `group()` does not pay.
- **The `String` key limit is stated in borsh bytes** (item 3 above).
- **A user-defined inherent `bind` on a v2 root** is E0592 duplicate definitions, as accepted. One that takes different arguments collides the same way. Not probed.
- **The hand-expanded `AccountView` in `state/tests.rs`** warns "field `nonce` is never read". The generated views in the integration test don't warn. Cosmetic.

### Test counts

- `near-sdk` lib `state::` unit tests: **56 passed**.
- `near-sdk/tests/state_v2_exchange.rs`: **19 passed** (+2: `group_mut_references_outlive_the_temporary`, `oversized_key_panics_naming_the_limit`), including 4 `should_panic`.
- `near-sdk` full lib (`unstable,abi,unit-testing`): **565 passed**, 1 ignored.
- `near-sdk` doc tests: **343 passed**, 6 ignored.
- `near-sdk-macros`: **49 + 1 passed** (2 + 2 ignored). Snapshots unchanged against master.
- `proto-wasm-check`: wasm32 check, native check and wasm32 release build all ok.

## Revision 3 results

Applied `/tmp/storage-v2-proto/revision-3.md` (the usability pass), commits `502d185b` (module, macro, renames, API) and `cc3cf0b2` (contract tests, root `Lazy::new`, new unit tests, success check), plus this notes commit. The three usability contracts (`state_v2_ft.rs`, `state_v2_nft.rs`, `state_v2_dao.rs`) are updated for every rename and use the new APIs where the revision says so.

### Success check

**Passes.** `proto-wasm-check/src/lib.rs` now holds the *revised* spec's `Exchange` verbatim (spec-rev.md, "What the developer writes"):
- `Map` for both `accounts` and `balances`;
- `required` on `owner`;
- the new `set_balance` via `at_mut`.

No other edit. wasm32 check, native check (`--features near-sdk/unit-testing`) and wasm32 release build all ok: 279,729 bytes, +9,012 over round 3. The extra is mostly `accounts` becoming the iterable `Map`, plus `set_balance`. The original spec's text with only the renames applied is a subset of this and also compiles.

### Per item

| # | Status | Reason |
|---|---|---|
| 1 | applied | `IterMap`→`Map`, `Map`→`LookupMap`, `IterSet`→`Set`, `Set`→`LookupSet`. The entry types are now `MapEntry` (for `Map`) and `LookupMapEntry`. Layout, kind bytes and tags are unchanged; every existing cost assertion passed untouched after the rename. **Consequence for v1 detection**, see objections. |
| 2 | applied | `Vec`→`Vector`. `state::*` no longer shadows `std::vec::Vec`. |
| 3 | applied | `{Name}At<'a>` and `{Name}AtMut<'a>`, each with generated rustdoc saying where it comes from and why its scalars are unavailable. |
| 4 | n/a | `Lazy` stays. |
| 5 | applied | `at_mut(&k)` on `Map` and `LookupMap` returns `{Name}AtMut<'a>`. It costs 1 `storage_has_key` (none if the key is already cached, e.g. after `contains_key`), panics on an absent record with "`at_mut` on a record that does not exist; create it with `entry(k).or_default()` (or `insert`) first", and reads nothing. Collection fields are `&'a mut` handles allocated in the round-2 cache arena (one per collection field of the record, per call); scalars are `UseGetMutForScalars`. The read-only hint uses `#[doc(hidden)]` stub methods (`insert`, `remove`, `get_mut`, `entry`, `push`, `set`, ...) on every `At<'a, C>`. Each stub is bounded on a never-implemented `ViewIsReadOnly` that carries `on_unimplemented`. `at(&k).balances.insert(..)` now says "views are read-only; use `at_mut` or `get_mut`" instead of suggesting `iter` (probe). The DAO `vote` moved to `at_mut`: **(7, 8, 0) + 1 has_key → (5, 8, 0) + 2 has_key**. It no longer reads the 2,000-byte proposal record (`vote_op_counts`, `read_bytes < 500`). |
| 6 | applied, extended | `Map::insert -> bool`. Also `Set::insert -> bool`, which now returns early on an existing key: 1 index read, 0 writes. Before, it rewrote the `()` entry. `LookupMap::insert` and `LookupSet::insert` stay `()`. The DAO double-vote check is `require!(proposal.votes.insert(voter, vote), "already voted")`. |
| 7 | applied | `page_with(after, limit, \|k, &v\| T) -> Page<K, T>` on `Map` and on `At<Map>`, where the closure can keep the `&'a V`. `page` is now `page_with(.., clone)`, and its rustdoc points at `page_with`. The DAO's hand-rolled `keys_page` + `get` is replaced. |
| 8 | **changed** | `Lazy::new(v)` holds `v` and writes it at `bind`, so `insert(id, Token { metadata: Lazy::new(m), .. })` works, and NFT `nft_mint` uses it. **As specified it loses data at the root**: an `#[init]` returning `Self { metadata: Lazy::new(m), .. }` without `.bind()` is never bound (the v2 `state_write` only flushes), so the value would vanish silently. I made the pending value a `Cell`, added a generated `__write_pending` walk, and `flush_root` writes pending root `Lazy` values through `&self`. NFT `new` now has no `.bind()`; test `init_lazy_new_at_the_root_is_written_without_bind`. Unchanged: `set` on an unplaced `Lazy` still panics "not bound", and a `Lazy::new` inside a record that is never placed writes nothing. |
| 9 | applied, extended | `Set` gets `key_at`, `page` (returns `KeyPage<K>`, since a unit value would be noise) and `iter`, and its view gets the same. `iter()`/`keys()` on `Map`, `Set`, `Vector` and their views return `IndexIter`, which overrides `nth`, `nth_back` and `count`. So `skip(n)` is free, and `count()` reads only the header. That broke one cache-memory test, which relied on `count()` touching every entry; it now uses `fold`. NFT `nft_tokens_for_owner` is `keys().skip(from).take(limit)`; test `offset_page_cost_does_not_depend_on_the_offset` shows offsets 0 and 5 cost the same. The module doc documents offset paging. |
| 10 | applied (`Map` only) | `Map::remove` with a zero-sized `V` skips the entry read and rebuilds the value from zero bytes (`decode_value(&[])`). Test `unit_valued_remove_costs_what_discard_costs`. NFT transfer and burn now use `remove` on the `()` group; `nft_transfer` still costs (8, 8, 3), the `discard` cost. `GroupedMap` docs call `GroupedMap<G, K, ()>` a grouped set. `LookupMap::remove` on a unit value still reads; `LookupSet::remove` already discarded. |
| 11 | applied | The derive rejects `Map<_, X>`, `LookupMap<_, X>`, `GroupedMap<_, _, X>`, `Vector<X>` and `Lazy<X>` when `X`'s last segment is a v2 collection name. The error says "a map value cannot be a collection; put it in a `#[near(storage)]` record, or use `GroupedMap`" and points at `X` (probe: the span is on `Set` in `Map<u32, Set<u32>>`). Syntactic, so an alias still reaches the post-mono backstop. |
| 12 | applied, reworded | `on_unimplemented` on `Removable`. The revision's text names only `LookupMap`, but the error usually fires on the *record* (`HasLookup: Removable`), and `LookupSet`/`GroupedMap` cause it too. Message: "`{Self}` cannot be removed or cleared: it is or holds a `LookupMap`/`LookupSet`/`GroupedMap`, which does not track its keys", with the note "use `Map`/`Set` ..., or keep the record and clear its fields by hand". The compiler still adds the "unsatisfied requirement introduced here" pointer at the derive (probe). |
| 13 | applied | Module rustdoc for `near_sdk::state`, under a page. It covers: <ul><li>the attributes and their options;</li><li>what the derive emits;</li><li>tags never reused;</li><li>`at`/`get`/`at_mut`;</li><li>once-per-call reads and ordered writes;</li><li>`contains_key` not warming the cache;</li><li>`remove` detaching;</li><li>enumeration order after a remove;</li><li>cursor panics → offset paging.</li></ul> |
| 14 | applied | `state::testing` is public (no `doc(hidden)`), with module docs naming the two harness traps from the usability notes. New items: `set_context(who, deposit)`, plus `Harness<T>` with `init(who, f)`, `attach()`, `call(who, deposit, f)` and `view(f)`. `view` asserts nothing is pending; `call` drops the root after the write, as the wrapper does. FT, NFT and DAO use it: about 30 copied lines each became three one-line wrappers. The exchange test keeps its own helpers, because its `mod legacy` v1-root tests need the env host and explicit drop timing. |

### Where I think the revision is wrong

- **Item 1 hands v1's names to v2 types, and that blinds the derive's v1 check.**
  - Round 2's "v1 collection in a record" rejection worked by last path segment. `LookupMap`, `LookupSet`, `Vector` and `Lazy` are now ambiguous: a record field written `LookupMap<_, _>` is a legitimate v2 field. A contract that did `use near_sdk::store::LookupMap;` produces exactly the same tokens and is no longer rejected.
  - I kept the check for v1-only names (`IterableMap`, `UnorderedMap`, `TreeMap`, `LazyOption`, ...) and for paths that name `store::`/`collections::`; probe: both rejected.
  - The unqualified case is now caught by nothing. A nested v1 `LookupMap` is a Borsh leaf, so it is blanket-`Removable` and `remove` orphans its entries (the round-2 test `removing_a_record_with_a_nested_v1_field_orphans_its_entries` still passes).
  - Migration friction is also likely: a file importing `near_sdk::store::LookupMap` and `near_sdk::state::*` gets a name clash. That one is by construction; I did not probe it.
  - If the names stay, the doc rule "v1 collections belong at the root" is now the *only* guard for the most common v1 type.
- **Item 1 also makes the spec example more expensive by default.**
  - In the revised `Exchange`, `accounts: Map<AccountId, Account>` is iterable. A first deposit by a new account goes from entry read + 1 write (round-2 `Map`, now `LookupMap`) to entry read + header read + 4 writes for that record. That is +1 read, +3 writes per new account, from the cost table (the exchange integration test still uses `LookupMap` for `accounts`, so this is not measured there).
  - That is the intended trade ("the default name is the safe type"). But the spec's headline example now pays it for a map nobody iterates, which reads oddly next to the cost claims.
- **Item 6: `insert -> bool` on a leaf has already overwritten when it returns `false`.**
  - `require!(votes.insert(voter, vote), "already voted")` is correct only because the panic aborts the receipt.
  - `if !map.insert(k, v) { log!("kept the old one") }` is wrong: the old value is gone (`map_insert_reports_whether_the_key_was_new`: `get` returns 11 after a `false`).
  - `std::collections::HashSet::insert -> bool` does not overwrite, and `HashMap::insert` returns the old value. This is neither. An `insert_new(k, v) -> bool` that does not write when the key exists would be the safe spelling for "first write wins". `Set::insert` now behaves that way, because the value is `()`.
- **Item 8 as written loses root `Lazy::new` values** (above). The fix needs interior mutability in `Lazy` and a flush-time walk. That is cheap, but it is a mechanism the revision didn't anticipate.
- **Item 5's "`&mut` handles" are allocations.** `at_mut` allocates one arena handle per collection field of the record, whether or not you touch them, freed at the flush. That is small and bounded per call, but a loop of N `at_mut`s on a record with 3 collections holds 3N handles until the end of the call.

### New friction and edge cases

- **`count()` is now free** (header only). That is an improvement, but anyone using `iter().count()` to warm the cache, as one of my own tests did, gets nothing.
- **The `At` view now has hidden write methods.** They exist only to carry the diagnostic. rust-analyzer hides `doc(hidden)` items from completion in other crates, as far as I know (not verified here).
- **`IndexIter` is a public type** in the return position of every `iter()`. Returning `impl Iterator` would have kept the `nth` override just as well. Naming the type is only so its docs can say "`skip` is free"; it could be hidden again without loss.
- **The `Harness::view` context.** It keeps the previous call's predecessor and deposit. A view that branches on `predecessor_account_id()` sees the last caller.
- **`at_mut` after `contains_key`** (both maps). After `contains_key(&k)` returned true, `at_mut(&k)` is free (`Orig::PresentUnread`). After it returned false, `at_mut` panics without a second host call. Both are as intended; the DAO `vote` relies on the first.

### Test counts

- `near-sdk` lib `state::` unit tests: **63 passed** (+7 revision-3 tests).
- `state_v2_exchange.rs`: **19 passed**.
- `state_v2_ft.rs`: **14 passed**.
- `state_v2_nft.rs`: **17 passed** (+2: `init_lazy_new_at_the_root_is_written_without_bind`, `offset_page_cost_does_not_depend_on_the_offset`).
- `state_v2_dao.rs`: **14 passed** (+1: `at_mut_on_absent_record_panics`).
- `near-sdk` full lib (`unstable,abi,unit-testing`): **572 passed**, 1 ignored.
- `near-sdk` doc tests: **343 passed**, 7 ignored (+1 ignored: the `testing` module example).
- `near-sdk-macros`: **49 + 1 passed** (2 + 2 ignored). Snapshots unchanged against master.
- `proto-wasm-check`: wasm32 check, native check and wasm32 release build all ok (279,729 bytes).
- Compile probes (scratch file, removed):
  - the read-only view hint;
  - the item-11 rejection at the field;
  - the `Removable` message;
  - `UseGetMutForScalars` on `at_mut(..).nonce += 1`;
  - `store::LookupMap` and `store::IterableMap` rejected in a record;
  - a v2 `LookupMap` accepted in a record.

## Migration round

The upgrade path from v1 was never prototyped before this round. Here it is tested against a real deployed v1 layout: the unmodified `examples/fungible-token`, populated through its own methods with 300 funded accounts and a 100 KiB icon in the metadata. It then gets two upgrades over the same storage: v1 → v2 ("move the root") → v3 ("rekey"). Both run on the mocked blockchain and on nearcore's runtime via near-contract-sim.

How to run:

```text
(cd examples/fungible-token && NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p fungible-token-v2)          # migration.rs (16), nested_trap.rs (1)
NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features unstable,unit-testing --test state_v2_migrate  # 5
(cd proto-migration-sim && CARGO_TARGET_DIR=~/Developer/near-contract-sim-scenario/target cargo +1.95.0 test --test ft_upgrade -- --nocapture --include-ignored)  # 2
```

Wasms in `proto-migration-sim/tests/res`, built with `cargo near build non-reproducible-wasm --no-abi`:

| wasm | built from | size | sha256 |
| --- | --- | --- | --- |
| `ft_v1.wasm` | `examples/fungible-token/ft` | 128,954 B | `de994ad5…` |
| `ft_v2.wasm` | `ft-v2` | 183,629 B | `eb451243…` |
| `ft_v3.wasm` | `ft-v2 --features v3` | 140,596 B | `c446ebaa…` |

### What was built

- **One ordered host log.** `env::storage_{read,write,remove,has_key}` feed an opt-in thread-local log (`state::testing::env_log_start/take/stop`, `env_store()`), gated on `unstable` + `unit-testing`, native only. The counting mock only sees the v2 cache, while v1 collections call `env` directly. So every same-storage test runs on `use_env_host()` plus this log. `MockedBlockchain::storage_snapshot` (crate-private) backs `env_store`.
- **`state::Migrating<V1, V2>`** (`near-sdk/src/state/migrating.rs`), with three traits:
  - `LegacyMap` (`collections::{LookupMap, UnorderedMap, TreeMap}`, `store::{LookupMap, IterableMap}`);
  - `LegacyIterable` (all of those except the two `LookupMap`s, with an `on_unimplemented` message);
  - `MigrationTarget` (v2 `LookupMap` and `Map`).
- **`examples/fungible-token/ft-v2`.**
  - `v2`: the v1 `FungibleToken` and `LazyOption` kept as root fields `#[key(1/2, required)]`, a new `#[key(3)] transfers_sent: Map<AccountId, u64>`, and a `#[private] #[init] migrate()` that reads the v1 `STATE` and removes it.
  - `v3`: `reserved(1)`, `FungibleToken` flattened, `accounts: Migrating<collections::LookupMap, state::LookupMap>`, and `#[private] #[init(ignore_state)] migrate()` from `raw_root`.
- **Macros.** No macro changes. The spec's `migrate(old: RawRecord)` hook is not a macro feature. It is an ordinary `#[init]` that calls `raw_root`.

### Claim 1, move the root: held, with changes

- **Migrate.** `migrate_moves_only_the_root`: the init guard's `has_key(F0)`, read `STATE` (34 B), remove `STATE`, then the flush writes `F0` (38 B). That is `(1, 1, 1)` plus 1 has_key, and nothing else. All 302 non-`STATE` v1 keys are byte-identical afterwards (mock and sim). The root holds tags 1 and 2 (tag 3 is a collection). Running `migrate` a second time fails with "The contract has already been initialized".
- **Reads.** `every_old_balance_reads_back_through_the_v2_root`: 300 users plus the owner. Each view costs 2 reads (root, balance) and never touches the 100 KiB metadata key `[01]`. Sim: all 300 match after `deploy v2 + migrate` in **one transaction** (13.455 Tgas for the call receipt, mostly the deploy).
- **The spec conflates `collections` and `store`.** `FungibleToken` and `NonFungibleToken` hold `near_sdk::collections::*`. These write through, with no cache and no write-on-drop. So in `transfer_between_old_accounts_after_migration` the v1 ops land **in the method body**:
  - body: `R F0, R 00‥a, W 00‥a, R 00‥b, W 00‥b`, then the new field's index and header reads;
  - flush: 4 v2 writes (new `Map` key), and no root write (no root scalar changed);
  - drop: nothing.

  Only `store::*` defers to `Drop`. `status_message::store_write_on_drop_lands_after_the_v2_flush` (the `examples/status-message` state, `store::LookupMap`) shows it: flush `W f002‥, W f0`, then drop `W 07000000‥u7.near`. That v1 write lands **after the root**.
- **Gas.** Sim, call receipt:

  | transfer | Tgas | r/w | contract loading |
  | --- | --- | --- | --- |
  | v1 | 1.673 | 3/3 (STATE write) | 140 G |
  | v2, sender's first (new `Map` key) | 2.204 | 5/6 | 200 G |
  | v2, steady | 1.878 | 4/3 | 200 G |

  - **v2 costs more per transfer than v1, not less.** From the profiles:
    - contract loading +60 G (129 → 184 KB wasm, paid by every method);
    - `WASM_INSTRUCTION` +50 G;
    - the new field +1 r, +1 w;
    - the `STATE` rewrite is gone, −1 w.
  - "3 r / 2 w without the new field" is inferred from the op logs, not measured.
- **New registrations still grow the v1 map** with the v1 bond (`new_registrations_after_the_upgrade_still_grow_the_v1_map`). "Add new fields as v2" does nothing for the map that grows.
- **Deploying without `migrate` bricks the contract until `migrate` runs.** Every call fails with `state: cannot decode root: Missing { tag: 1 }`, because the `required` v1 fields are missing from the empty root (`deploy_without_migrate_window`, `v2_root_is_absent_until_migrate_runs`). `migrate` is `#[private]`, so a stranger gets "Method migrate is private".

  Any *public* `#[init]` on the v2 contract would be callable in that window, because the guard checks the v2 root. Deploy and migrate belong in one transaction; this works on the sim.

### Claim 2, root byte never collides: held for the real contracts, broke under a reachable shape

- **The real contracts are clean.**
  - FT v1 first bytes are `{00, 01, 'S'}`. After both upgrades they are `{00, 01, F0}` (mock and sim).
  - status-message has an **empty** prefix (unit-struct `BorshStorageKey`). Its keys are borsh(AccountId), so the first byte is a length ≤ 64.
- **Broke.** `root_byte::empty_prefix_v1_map_with_caller_keys_can_write_a_v2_key`:
  - Setup: a v1 `store::LookupMap<Vec<u8>, Vec<u8>>` with an empty prefix next to a v2 `LookupMap<String, u128>` at tag 1.
  - Construction: a 496-byte v1 key serializes as `F0 01 00 00 ‥`. That is the v2 entry key of a 477-char handle whose h16 starts with `00`, found in about 256 hashes. A 12-byte v1 value is 16 Borsh bytes.
  - Result: the attacker's v2 balance reads `340282366920938463463374607427473244172`.
  - **Preconditions.** All three must hold:
    1. a v1 map at the root with an **empty** prefix;
    2. caller-chosen variable-length v1 keys. With `Vec<u8>` keys the attack works as shown. With `String` keys the 15 random h16 bytes must also be UTF-8, about 2^15 more grinding.
    3. a v2 map at **tag 1** with `String`/`Vec<u8>` keys. At tag ≥ 2 the v1 length prefix reads ≥ 752, so the v2 key would need a 733-byte handle, over `StorageKey`'s 512-byte bound. That bound guards by accident.
  - **Immune:** an `AccountId`-keyed v2 map. Its key is at most 87 bytes, and the v1 key's length prefix can never read `F0 ..` at that size.
  - None of the three guards helps. The default byte only matters for enum prefixes, and a configurable byte is aliased the same way. The state scan is point-in-time and cannot see keys written later.
  - **Suggested rule:** check the v1 *prefixes* instead of the keys. Every v1 field kept at a v2 root needs a non-empty prefix whose first byte is not the root byte. The prefixes are finite and sit in the root record, so this can be checked at migrate time.

### Claim 3, rekey with `Migrating`: held, with changes

- **Read fallback** (`migrating_read_falls_back_with_two_reads_per_miss`):
  - an old account costs root + new miss (0 B) + old hit, i.e. 2 reads below the root;
  - a key on neither side costs root + 2 misses.
- **Write forward** (`migrating_write_moves_the_key_and_deletes_the_old_entry`):
  - first transfer between two old accounts: `(7, 6, 2)`, which includes 4 writes for a new `transfers_sent` key;
  - the two v1 entries are removed in the body (write-through) and the two v2 entries written at the flush;
  - next transfer: `(4, 3, 0)`, with no old-side access.
  - Sim: moving 2.176 Tgas (6/3/2), migrated 1.876 Tgas (4/3/0).
- **`migrate_batch` on a v1 `LookupMap` is a compile error.** With the bound on the method (it started on the impl, which gave a bare E0599 without the message):

  ```text
  error[E0277]: `near_sdk::collections::LookupMap<near_sdk::AccountId, u128>` cannot enumerate its keys, so `Migrating` cannot move its entries in batches
    = note: a v1 `LookupMap` stays on the lazy path permanently: reads fall back to it and each write moves one key; only an off-chain key list (an indexer) can drain it
  ```

- **Iterable v1 maps drain and the wrapper comes off.** `migrate_batch_drains_iterable_v1_maps_and_the_wrapper_comes_off` uses NFT's `owner_by_id: collections::TreeMap<String, AccountId>` shape plus a `store::IterableMap`:
  - 120 + 120 entries move in 5 calls of `migrate_batch(25)` each;
  - the first batch costs `(1437, 437, 100)`, almost all of it TreeMap rebalancing, so batch size has to be tuned to gas per v1 type;
  - afterwards no v1 key is left;
  - redeployed with bare `Map`s at the **same tags**, all entries read back with 0 writes;
  - but the root still carries the stale v1 prefixes under tags 1 and 2 (48 bytes), because the encoder keeps stored bytes for a known non-inline tag. Harmless, but permanent.

  `write_then_batch_keeps_the_newer_value`: a write through the wrapper followed by batches keeps the newer owner.
- **`Migrating<store::LookupMap, Map>`** (`store_lookup::store_lookup_map_delete_lands_at_drop_after_the_v2_flush`). This is the only deferred legacy delete.
  - body: reads only;
  - flush: the v2 writes, then the root last;
  - drop: `Remove` of alice's v1 entry, plus a `storage_remove` for `bob`, who was never on either side. A write of a key not seen on the new side always pays one blind v1 remove.
- **Storage grows on every move.** An account key goes from 17 B (v1, `user001.near`) to 35 B (v2), +18 B paid by the contract, not the user. The v1 NEP-145 bond (`account_storage_usage` = 125 B, measured for v1 keys) undercharges a v2-side registration, which uses 143 B (`v1_storage_bond_undercharges_v2_keys`).
- **`FungibleToken` cannot be wrapped.** Its `accounts` is a concrete `collections::LookupMap` field of a standards struct. v3 flattens it and re-implements `ft_transfer`, `ft_balance_of` and registration. `NonFungibleToken` has the same problem.

### Claim 4, nested v1 is a trap: held (the trap is real and wider than stated)

- Qualified `near_sdk::store::LookupMap` in a record: rejected, as before.
- **Unqualified `use near_sdk::store::LookupMap;` next to `use near_sdk::state::*;`.** The explicit import silently shadows the glob. Every `LookupMap` in the file is then v1, including in `#[near(storage)]` records, with no error and no warning (`nested_trap.rs`, `mod shadowed`, which compiles and type-checks as v1). Round 3 predicted a name clash; there is none.
- `removing_a_record_that_holds_a_fungible_token_orphans_its_balances`: a standards `FungibleToken` in a record is accepted (an opaque name), the record is blanket-`Removable`, and `remove` leaves alice's balance under `p1‥`. `collections::` nested in a record still *writes* (write-through); only `store::` writes late.

### Sharpest edge

**Every near-contract-standards token hits this when it rekeys.** Step 1 (move the root) is clean: one record rewritten, 302 keys untouched, deploy and migrate in one transaction. Step 3 is where it gets expensive. The accounts map is a concrete field of `FungibleToken`, so it cannot be wrapped: the contract forks the standard (v3 re-implements transfer, balance and registration).

Then every lazily moved account grows its key from 17 B to 35 B, paid from the **contract's** balance, because `ft_transfer` attaches 1 yocto and no storage deposit. Take a token with 1M holders: about 18 MB, or about 180 NEAR of staked storage, drained a transfer at a time. Meanwhile new registrations keep paying the v1 bond (125 B) for a 143 B record. Nothing in the design re-measures the bond.

Runners-up:
- **The brick window.** Every method panics with `Missing { tag: 1 }` between deploy and migrate. One-transaction deploy + migrate avoids it (sim).
- **Root-byte aliasing.** Claim 2 above: conditional, but it reopens the exploit class v2 was meant to close.

### Every change and every invention

- `env.rs`: the op log; `MockedBlockchain::storage_snapshot`; `testing::env_log_*`, `env_store`. Off by default; the full lib suite passes (565).
- `Migrating` layout (invented):
  - the v1 half is inline in the parent record and the v2 half sits at `record ++ tag`;
  - `INLINE = true` with `MAX_SUBPATH = V2::MAX_SUBPATH`;
  - must be `required`.
- Tag changes (invented): v1 root field → `Migrating` is a **new tag** plus `reserved(old)`; `Migrating` → bare map is the **same tag**.
- `Migrating` API (invented):
  - `get` returns an owned `V: Clone`, because the v1 side doesn't cache;
  - `insert` checks the new side first (a cache hit after `get`) and skips the old-side delete if the key is there (one-side invariant);
  - `store::LookupMap` deletes via `set(k, None)` (lands at its `Drop`);
  - `collections::LookupMap` deletes via `remove_raw` (1 `storage_remove`, no read).
- `migrate` (invented):
  - v1 → v2 removes `STATE` (the spec is silent; keeping it makes a rollback read a stale `total_supply`);
  - v1 → v2 is a plain `#[init]`, which is idempotent by construction;
  - v2 → v2 needs `#[init(ignore_state)]` plus a hand guard (tag 1 present), because the root exists.

### Not done

- **The real NFT contract.** Only its `owner_by_id` TreeMap shape and a `store::IterableMap` were tested; `NonFungibleToken` needs the same fork.
- **Untested types and paths.** `collections::UnorderedMap` (implemented, untested); a real `store::LookupMap` contract (only the synthetic `store_lookup` test); `Vector`/`LazyOption` rekeying; `ft_transfer_call`/resolve under v2.
- **Not on the sim.** `migrate_batch`, and the aliasing attack.
- **Not built.** The `migrate(old: RawRecord)` macro hook, and dropping stale inline bytes when a tag turns non-inline.

## API round (Oct 17)

This round brings the prototype in line with the settled decisions in `decisions-oct17.md`, items D2 to D9. D1, D10, G* and M* are doc-only and were not acted on. It starts by merging `proto/storage-v2-gas`, which was clean: the `unstable-state-hash8` feature, `proto-gas/` and `GAS_MEASUREMENTS.md`, with the lib suite at 566. Each decision also has a line in the `state` module docs (`near-sdk/src/state/mod.rs`).

| | What changed | Tests |
| --- | --- | --- |
| D2 | `#[key(n, default = expr)]`. A missing tag evaluates `expr` lazily, in decode and in the generated `Default`, and the type needs no `Default`. `required` together with `default` is a derive error. `RecordError::Missing` now displays `required field tag N is absent ... needs #[key(N, default = ..)] ...`. | `key_default_expr_fills_missing_tags`, `key_required_still_decodes_and_fails_when_missing`, `field_added_with_default_decodes_an_old_record`, `field_added_as_required_fails_an_old_record_naming_the_tag` (exchange) |
| D3 | `GroupedMap` (with `Group`, `GroupMut`) is `#[doc(hidden)]` with a deferral note. There is no `#[deprecated]`, because the NFT test still uses it and the attribute would spray warnings there. It is gone from the module docs and from the derive and `CHECK` messages. Exchange is rewritten to `markets: Map<MarketId, Market>`, and `proto-wasm-check` carries the same text. | `markets_place_and_cancel`, `market_record_scalars_and_orders`, `opening_a_market_twice_panics`, `cancel_in_an_absent_market_panics`, `place_off_tick_panics` |
| D4 | No code change: the prototype already preserved unknown tags and dropped only `reserved` ones. | `unknown_tags_survive_get_mut_and_entry_reserved_tags_are_dropped`, `hazard_reusing_a_removed_tag_resurrects_its_stale_bytes` |
| D5 | Under `unit-testing`, `env::storage_usage()` panics when the v2 cache has unflushed changes (any slot not Clean). The message points at `storage_delta()`. New `Receipt::storage_delta()` and `testing::storage_delta()`, using key + value + 40 per record (`STORAGE_RECORD_OVERHEAD`). Removes now record `replaced_len = Some(old)` when the key existed. | `storage_usage_inside_a_v2_mutating_method_panics`, `storage_delta_is_the_post_flush_storage_change` (ft; the mock delta equals the mocked blockchain's `storage_usage` delta, and unregister returns it) |
| D6 | (Superseded in part by API round 3, T1 and T7, below.) `insert` always overwrites and requires `V: Leaf`. `Leaf` is a new trait: a blanket over Borsh, and derived with HRTB bounds when no field owns storage, the same trick as `Removable`. New `insert_new(k, v) -> &mut V` requires `V: Record` and panics if the key is present. `Map::insert -> bool`, `LookupMap::insert -> ()` with no existence read. The rejected call is E0277 with the fix in the note. | `compile_fail,E0277` doctest plus the passing fix on `LookupMap::insert`; `insert_new_returns_the_bound_record`, `insert_always_overwrites_a_leaf_record`, `insert_new_over_existing_record_panics` |
| D7 | `at_mut(&k) -> Option<NameAtMut>` on `Map` and `LookupMap`. The DAO `vote` lost its `contains_key` check (the `expect` is the check). | `at_mut_on_an_absent_record_is_none` (lib and DAO), `vote_on_absent_proposal_panics`, `set_balance_on_an_absent_account_panics` |
| D8 | `page`, `page_with`, `keys_page` and `Set::page` return `Result<_, CursorGone>`. `CursorGone: AsRef<str>`, so a `#[handle_result]` view returns it as is; the Exchange `balances` does, and the wasm32 check covers that wrapper. The offset-paging skip is documented on `Map::page`. | `page_with_removed_cursor_returns_cursor_gone`, `offset_paging_skips_a_key_after_a_removal_behind_the_offset`, `balances_with_a_removed_cursor_is_cursor_gone` |
| D9 | `Map/LookupMap::remove_some(&k, n) -> bool` (true while entries remain) removes a record in bounded steps. New `Removable::clear_children_some(path, &mut budget)`, which the derive emits. While a clear is unfinished, the record bytes start with the clearing flag, and `get_mut`, `entry` and `at_mut` on that record panic; `get` and `at` still read. Format decision: the flag is `00 00` (tag 0, length 0, first). | `clear_across_two_calls_refuses_an_interleaved_insert`, `writes_below_a_clearing_record_panic_in_the_same_call`, `clear_that_fits_in_one_call_writes_no_flag` (exchange); `remove_some_budgets_across_calls`, `clearing_flag_is_a_leading_empty_tag_zero` (lib) |

### Invented where the decisions are silent

- **D2:** `default` and `required` together are rejected. A `default` on a collection field is a post-monomorphization `const` assert.
- **D3:** the `Market` scalars are `tick_size: u128` and `next_order_id: OrderId`, at tags 2 and 3, with `orders` at tag 1. The five spec method bodies match `proto-wasm-check/src/lib.rs`:
  - `set_balance`: `at_mut(&id).expect("no such account")`.
  - `cancel`: `markets.at_mut(&market).expect("no such market").orders.remove(&order)`.
  - `balances`: `#[handle_result] -> Result<Page<TokenId, u128>, CursorGone>`.
  The doc block should be reconciled with these.
- **D5:** the guard keys on cache state, not on a "v2 call active" flag. That way views and code between calls never trip it (the ft-v2 migration test reads `storage_usage` right after a view), and any "after" read in a method that wrote does trip it. Gap: a change to a root scalar alone does not trip it, because the root is encoded only at the flush.
- **D6:** `Lazy::set` on a record that owns collections is still the runtime insert-new check, not a trait bound. `MigrationTarget` now requires `V: Leaf`; values migrated from v1 are always Borsh.
- **D8:** `keys_page` and `Set::page` also return `Result`, for consistency.
- **D9:**
  - The API name is `remove_some(&k, n)`. `clear(n)` would collide with `Map::clear()`, and the operation is a bounded remove.
  - The budget counts entries removed at any depth. `Lazy` values do not count.
  - `n = 0` clears nothing and flags the record.
  - `replace` and `remove` on a clearing record are allowed: they clear unbounded and drop the flag.
  - Only values with `MAX_SUBPATH > 0` consult the flag, because a Borsh leaf may legitimately start with `00 00`.
  - Reconciled Oct 17: `remove_some(&k, n)` stays. It sits on the same receiver as `Map::clear()`, so the doc and `docs/storage-v2-spec.md` adopt it and record the choice in Open questions.

### Cost change the designer must see (D9)

Superseded by "API round 2 (Oct 17, flag key)" below: the flag moved to its own key and these costs are reverted.

The flag lives in the record bytes, which `storage_has_key` cannot see. So `at_mut` now does 1 read of the record (no decode) instead of 1 `storage_has_key`. Every `at_mut` path has +1 read and −1 has_key, and pays the record's bytes:

- `set_balance`: (2,1,0) + 1 has_key → (3,1,0).
- `cancel`: (5,3,3) → (6,3,3).
- DAO `vote`: (5,8,0) + 2 has_key → (7,8,0) + 1 has_key, and the 2,000-byte proposal description is read again. The round-3 win, "the proposal record is no longer read to insert a vote", is gone.

The doc's "one existence check, no decode" for `at_mut` and `set_balance` is now half true: there is no decode, but there is a read.

Alternatives:

- A sibling flag key checked with `storage_has_key`. Each `at_mut` then costs 2 has_key calls, record and flag.
- A flag checked only at the flush, for keys under an `at_mut` path. It still needs the read, just later.
- Accept the cost.

### Test counts (end of round)

- Lib: 571 (566 after the merge).
- Doctests: 344, including the D6 `compile_fail` case.
- `state_v2_exchange`: 34. `state_v2_ft`: 16. `state_v2_nft`: 17. `state_v2_dao`: 15. `state_v2_migrate`: 5.
- ft-v2 example: 16 migration plus 1 nested_trap.
- `proto-wasm-check`: wasm32 check passes.

`abi_tests` and `deny_unknown_arguments_tests` fail in this worktree for environment reasons: a workspace-detection error building `examples/adder`, and no real sandbox.

### Not built

- The deployed-ABI `storage diff` checks: D2's hard fail on a newly added `required` tag, and D4's build error on tag reuse.
- An on-chain `env::storage_delta()`.
- A trybuild `.stderr` snapshot for D6. The doctest checks E0277 only; the message text is quoted in the doctest's comments.

## API round 2 (Oct 17, flag key)

The designer's decision: the `remove_some` clearing flag leaves the record's bytes and becomes a separate key beside the record, checked by existence, so that `at_mut` is back to one existence check with no read and no decode. Branch `proto/storage-v2-d9`, from `proto/storage-v2` at 77f63c76.

### Key layout

The flag key is the record's key plus one byte, `00`:

```text
flag_key := record_key 00
markets[7] record: F0 04 00 h16(7) 07000000      23 bytes
markets[7] flag:   F0 04 00 h16(7) 07000000 00   24 bytes
```

- **Not a field below the record:** `00` is tag 0 as a LEB128 varint, which no `#[key]` may use (`Path::field(0)` asserts), and every other tag's varint starts with a nonzero byte.
- **Not a sibling entry:** map keys are prefix-free Borsh behind their own hash, so no `h16(k') borsh(k')` equals `h16(k) borsh(k) 00`.
- **Sorted beside its record:** it comes right after the record key and before every child, so the flush stays in key order.
- **Within 2,048 bytes:** it is one byte longer than the record key and shorter than any key below the record. A record that owns collections always has a longer child, so the derive's 2,048-byte sum already covers it, and `Path::checked` asserts at runtime too.

In code it is `key::FLAG_SEGMENT` and `Path::flag()`, with the layout in the `key.rs` module docs and `docs/storage-v2-spec.md` (Keys are paths from the root, the Flag row). Record bytes carry no flag any more. `record::CLEARING_FLAG`, `record::is_clearing` and the `parse` skip are gone, so tag 0 in a record is `TagZero` again, `00 00` included.

### Existence-check scheme: the record moves to the flag key

A `storage_has_key` returns one bit, and `at_mut` has to tell three states apart: absent, present, being cleared. One check on the common path is only possible if "record key present" by itself means "not being cleared". So while a clear is unfinished the record's bytes, unchanged, live at the flag key and the record key is absent. This is a step past "a flag key beside the record": the record relocates for the duration of the clear. It is the option the decision called folding the check into the key already checked.

- **Hit at the record key:** present and not clearing. `at_mut`, `get_mut` and `entry` pay nothing for D9, so their costs equal those before D9 existed.
- **Miss at the record key, for a map value that owns storage (`MAX_SUBPATH > 0`):** consult the flag key. `get`, `entry`, `remove`, `replace` and `remove_some` read it, because they need the bytes. `contains_key`, `insert_new` and `at_mut` check its existence. Leaves, `Vector` slots and `Lazy` never consult it.
- **Flush:** `Clearing::Set` writes the encoding to the flag key and removes the record key. A record loaded from the flag key and then replaced (`Clearing::Cleared`) writes the record key and removes the flag key. A delete removes whichever key holds the record. A blind delete (`discard`, a bulk `clear` of a map of records) removes both, because a nested record may itself be mid-clear.
- **Refusals:** `LookupMap::insert_new` over a clearing record panics with "is being cleared" (new, free: the slot knows). `Map::insert_new` still says "record exists", because the index is still there.

The rejected alternative keeps the record at its key and checks the flag key on every write that hits an existing record. That gives `at_mut` 2 checks, and adds 1 check to `get_mut` and `entry` on existing records: a repeat `deposit`, `place`, `fill` and `cancel` would each pay it, and DAO `vote` would be 4 checks. Moving the record instead charges misses, which are mostly record creation and happen once per record.

### Before and after, in reads, writes, removes + existence checks

| Method | Before D9 (rev 3) | Flag in record (round 1) | Flag key (now) |
| --- | --- | --- | --- |
| Exchange `set_balance` | (2,1,0) + 1 | (3,1,0) + 0 | **(2,1,0) + 1** |
| Exchange `cancel` | (5,3,3) + 1 | (6,3,3) + 0 | **(5,3,3) + 1** |
| Exchange `cancel` of an unknown order | not recorded | (3,0,0) + 0 | **(2,0,0) + 1** |
| Exchange `fill` | not recorded | (3,1,0) + 0 | **(2,1,0) + 1** |
| DAO `vote` | (5,8,0) + 2 | (7,8,0) + 1 | **(5,8,0) + 2**, proposal not read (< 2,000 bytes read) |
| lib `at_mut` + insert below (`Map<u32, Account>`) | not recorded | (3,4,0) + 0 | **(2,4,0) + 1** |
| Exchange first `deposit` (new account) | (5,8,0) | (5,8,0) | **(6,8,0)**: the flag key read on the account miss |
| Exchange `new_seeded` init | (2,5,0) | (2,5,0) | **(3,5,0)** |
| lib first `deposit` into a `LookupMap` record | (4,6,0) | (4,6,0) | **(5,6,0)** |
| `LookupMap::insert_new` of a record | 1 check + 1 write | same | **2 checks** + 1 write |
| `at_mut` on an absent record | 1 check | 1 read | **2 checks** |
| bulk `clear` / `clear_some` / `discard` of a map whose values own collections | 3 removes per entry | same | **4 removes per entry**: the flag key, blind |
| repeat `deposit`, `open_market`, `place`, FT, NFT, DAO finalize and remove_member | unchanged | unchanged | unchanged |

The DAO finalize (33,0,44), remove_member (29,3,40), nft_transfer (8,8,3) and nft_burn (10,2,17) counts printed by the suites are identical at 77f63c76 and now. Every op-log assertion above is exact in the test suites: `state_v2_exchange`, `state_v2_dao` and the lib `state::tests`.

Clearing, per call: 1 record read (2 once flagged), the clearing work, then on the first unfinished call 1 remove (record key) + 1 write (flag key), and on the finishing call 1 remove of the flag key. A flagged record stakes 1 more key byte.

### Example and doc

- Exchange `Market` gets `#[key(4, default = true)] open: bool,        // added later; older records read as open` in both `proto-wasm-check/src/lib.rs` and `near-sdk/tests/state_v2_exchange.rs`. The struct block and the five spec methods are byte-identical between the two. Its one use is in the test file's `place`, outside the printed block: `assert!(m.open, "market closed")`, plus a `halt_market` helper. Test: `market_open_defaults_to_true_for_an_old_record`, where a record seeded without tag 4 reads as open and a halted market refuses an order.
- The `set_balance` comment, "writes below the record without reading it: one existence check, no decode", is accurate again under the flag key: measured (2,1,0) + 1 check. So it is unchanged, and the spec's caveat that it was stale is removed.
- `docs/storage-v2-spec.md`:
  - Keys: a Flag row in the segment table, and a paragraph with the `markets[7]` example.
  - Cost table: the `LookupMap` `get` and insert columns, and the `Map` insert-new column.
  - Bounded clearing: rewritten.
  - "`at_mut` is one existence check": replaces "`at_mut` reads the record", with the before/after table.
  - Two ways to write; the API-round paragraph; the prototype test counts.
  - Exchange listing: the `open` line.
  - Open questions: the `at_mut` row is decided.

### New tests

- `flag_key_is_the_record_key_plus_a_zero_byte` (key).
- `tag_zero_is_rejected_even_leading_and_empty` (record), replacing `clearing_flag_is_a_leading_empty_tag_zero`.
- `flag_key_is_dropped_by_replace_remove_and_discard` (lib). It also covers `insert_new` and `entry` refusing a clearing record, and `Map::iter` yielding it.
- `market_open_defaults_to_true_for_an_old_record` (exchange).
- Rewritten: `clear_across_two_calls_refuses_an_interleaved_insert`, `clear_that_fits_in_one_call_writes_no_flag` and `remove_some_budgets_across_calls` now assert that the record key is absent and the flag key holds the unchanged bytes, and that a one-call clear never touches the flag key. The interleaved-insert refusals and the `get` mid-clear are as before.

### Deliberately not done

- No flag check at the flush. It would still be one host call per record, only later.
- No change to `get`/`at` semantics mid-clear. `get` still returns the record, which is why a `get` miss on a map value that owns storage pays the flag-key read. Making `get` return `None` mid-clear would remove that read, but iteration would then meet an indexed key with no entry.
- `entry` and `get` read the flag key on a miss rather than checking its existence. The bytes are needed for `get` and for a continuing `remove_some`, and one shared path keeps the cache simple. A miss is 1 read of an absent key, not a `storage_has_key`.
- No migration path for records written with the old `00 00` prefix. The prototype never shipped. Such a record fails closed with `TagZero` on `get`, `get_mut` and `entry`, but **not** on `at_mut`, which only checks the record key (review F2; corrected in API round 3): storage written by earlier prototype builds must be reset.
- No change to `Map::insert_new`'s message mid-clear ("record exists" from the index). Telling it apart would cost a host call.
- The Claude Doc is not touched; it syncs from this section and the spec.

### Test counts (end of round 2)

- Lib: 573 (571 + 2 new; the record test was renamed).
- Doctests: 344.
- `state_v2_exchange`: 35. `state_v2_ft`: 16. `state_v2_nft`: 17. `state_v2_dao`: 15. `state_v2_migrate`: 5.
- ft-v2 example: 16 migration plus 1 nested_trap.
- `proto-wasm-check`: wasm32 check passes, with no warnings.

## API round 3 (Oct 17)

This round applies round-2 decisions T1 to T7 from `decisions-oct17-round2.md` on branch `proto/storage-v2-d9` (from 386d70e2), one commit per decision, then the four fixes from the independent D9 review (`codex-d9-review.md`, F1 to F4) and the `flag_key` bounds check. T8 waits on an audit and T9 to T11 are doc-only; none of them was acted on. `docs/storage-v2-spec.md` carries the same changes in place and a round-3 table under Rollout.

### T1: `insert` overwrites, `try_insert` is first-write-wins

- `Map::insert -> bool` is `#[must_use]`, with a message naming `try_insert` and `let _ = ..`. `LookupMap::insert -> ()` is unchanged.
- `insert_new` is renamed `try_insert(k, v) -> Result<&mut V, Occupied>` and has no `V: Record` bound, so it exists for leaves too. Collection-owning records still have no `insert` (the `compile_fail,E0277` doctest stays, its note now names `try_insert`).
- **Chosen where the decision is silent.** The handle is `&mut V`, as `insert_new` returned. `Occupied` is a unit struct with `Debug`, `Display`, `Error` and `AsRef<str>` (like `CursorGone`). It does not hand the value back: `Result::expect` needs `E: Debug`, and records do not derive `Debug`, so `try_insert(..).expect("..")` would stop compiling for exactly the types that need it. A record being cleared is `Err(Occupied)` on `Map` (its index is still there, as before) and still a panic ("is being cleared") on `LookupMap`.
- **Costs.** `Map::try_insert` = `Map::insert`: 1 index read, then 1 header read + 4 writes for a new key; an existing key costs the index read and nothing else. `LookupMap::try_insert` is 1 `storage_has_key` + 1 write for a leaf (`insert` is 1 write), and 2 checks + 1 write for a collection-owning record (unchanged from `insert_new`). An occupied key never reads the stored value.
- **Call sites.** DAO `add_member` drops its `contains_key` (`require!(members.try_insert(..).is_ok(), "already a member")`), DAO `vote` is `require!(p.votes.try_insert(..).is_ok(), "already voted")`, NFT `nft_mint` drops its `contains_key`, and Exchange `open_market` is `try_insert(..).expect("market exists")`. No DAO or NFT op-log assertion covers `add_member` or `nft_mint`, so no count changed; each saves 1 `storage_has_key`.
- **The Exchange listing changed:** `set_balance` discards the bool on purpose, so its line is now `let _ = self.accounts.at_mut(&id).expect("no such account").balances.insert(token, amount.0);` in the spec, `proto-wasm-check` and `state_v2_exchange.rs`. Every other ignored `Map::insert` in the tests got `let _ =` (43 sites, plus one in `Migrating::insert`); the wasm32 check has no warnings.
- Tests: `try_insert_returns_the_bound_record`, `try_insert_over_existing_record_is_occupied` (was `insert_new_over_existing_record_panics`; asserts (1,0,0), the index only), `try_insert_on_leaf_values_keeps_the_first_write` (new).

### T2: accessor names stay, the rule is written down

No code change. The spec's "`at` walks, `get` reads, `at_mut` writes below" paragraph now opens with the rule: `at*` is path arithmetic plus at most one existence check and never decodes; `get*` reads and decodes. It holds in the code as measured in round 2: `at` reads nothing, `at_mut` is 1 `storage_has_key` on a present record (2 on an absent record that owns collections, the D9 flag key, which is still "at most one" per key looked at but two host calls; the spec's cost table states it).

### T3: `remove_step(&k, n) -> Removal`

- `Map::remove_some` and `LookupMap::remove_some` are renamed `remove_step(&k, n) -> Removal`, with `#[must_use] pub enum Removal { Pending, Done }` (exported from `state`; `Debug, Clone, Copy, PartialEq, Eq`, plus `Removal::is_done()`). The must-use message says `Pending` means "call again in a later call".
- **Chosen where the decision is silent.** The `LookupMap` twin is renamed too, for one name. An absent record is `Done` (was `false`). The panic on a write below a clearing record now says "being cleared by `remove_step` ... until it returns `Removal::Done`". The Exchange test's `close_market` still returns `bool` to JSON (`== Removal::Pending`), so `Removal` needs no serde.
- **The self-contradiction.** `docs/storage-v2-spec.md` was consistent ("`true` while entries remain" in both places); the contradiction the decision names is in the Claude Doc's appendix C, which this round does not touch. With an enum the direction cannot be misread.
- No cost change. Tests: `remove_step_budgets_across_calls` (renamed) and the clearing tests now assert `Removal::Pending` / `Removal::Done`.
- Superseded wording: "`remove_some`" in the API round and round 2 sections above is the old name.

### T4: `revision` on every page

- `Page` and `KeyPage` gain `pub revision: u64`; `Map::revision()` (and on an `At<Map>` view) reads it. It comes from the map header, which every page reads already for the length, so pages cost the same reads as before (`page_revision_changes_only_across_a_removal` asserts header + 2 slots + 2 entries).
- The header is now `collections::MapHeader { len: u32, revision: u64 }`, with its own `Value` impl: the bare little-endian `u32` (the old 4-byte header) while `revision == 0`, and 12 bytes (`len ++ revision`) after. A map that never had a removal is byte-identical to before. `Vector` keeps its bare `u32` header.
- **Is the header already written on every removal? Almost.** `remove`/`discard` (via `remove_inner`), `clear_some` and the bounded clear all call `set_len_at`, but `set_len_at(.., 0)` *removed* the header. A counter that dies when the map empties resets to 0, so a page taken before the map emptied and one taken after it was refilled could carry equal revisions with removals in between. I kept the counter alive instead: the call that empties a map now **writes** the header (`len = 0`, revision kept) where it used to remove it. Same number of host calls (a write in place of a remove), but the header key stays staked: key + 12 + 40 bytes, per emptied map, until the record owning the map is removed (`Removable::clear_children[_some]` for `Map` now deletes the header, so removing a record still leaves nothing below it). The alternative, removing the header and documenting that revisions reset on empty, is one line to switch back to if the designer prefers no persistent empty headers.
- Bumps: `remove_inner` +1 per removal; `clear_some` +1 per call that removes anything (it now returns early when there is nothing to remove, instead of rewriting the same length); the bounded clear +1 per entry (one write at the flush either way); `push_key` keeps it.
- **Op-count changes (all from the empty-map header):**
  - lib `itermap_remove_last_key_has_no_moved_key`, emptying: (3,0,4) → (3,1,3). `grouped_map_groups_are_independent_itermaps`, last entry of a group: (3,0,4) → (3,1,3).
  - lib `bounded_clear`: the map ends with its header (len 0, revision 3) instead of no keys. `itermap_remove_cost_is_independent_of_neighbor_value`: the largest value a host call moves is the 12-byte header (was ≤ 8).
  - DAO `finalize` with 5 votes: (33,0,44) → (33,7,37), the 5 voters' `votes` sets, the proposer's `proposals` set and the root `proposals` map each emptying. `remove_member`: (29,3,40) → (29,8,36): 5 removes become header writes, plus 1 more remove for the removed member's empty `votes` header that the earlier `finalize` left behind.
  - DAO `finalize_counts_and_removes` / `remove_member_cleans_up_proposals_and_votes` and NFT `burning_the_last_token_empties_the_owner_group` asserted "no key left behind"; they now assert that the only new keys are emptied maps' headers (4, 4 and 2 of them: the NFT's are the root `tokens` map and alice's owner group, which, being a `GroupedMap` group, is never removed).
  - Unchanged: every Exchange, FT and migration count, NFT `nft_transfer` (8,8,3) and `nft_burn` (10,2,17), and every 4-byte header byte count in tests (no removal had happened).
- Tests: `page_revision_changes_only_across_a_removal` (new): two pages with only an insert between them have equal revisions; a removal changes it; emptying with `clear()` and re-inserting the cursor key still gives a different revision; the header is 12 bytes after a removal and 4 without; removing the owning record removes the emptied map's header.

### T5: hash width stays 16; the storage comparison is in the spec

No code change. The spec's "Entry hash width" paragraph now has the storage side: at 8 bytes a nested `Map` entry saves 40 B (16 in the value key and 16 in the index key, each holding two hashes, 8 in the key slot, which holds the parent's hash), about 400 NEAR per million entries at 1e19 yocto/byte, and the reasons 16 stays: the width can never be widened; birthday bound n²/2^65 at 8 bytes is 2.7e-8 at 1M and 0.027 at 1B; correctness never depends on the hash because the Borsh key follows it; trie-prefix grinding costs 2^b per hashed byte at either width.

I checked the 40 B against `key.rs` rather than waiting on the audit: the value and index keys of `accounts[alice].balances[usdt]` are `F0 03 00 h(alice) borsh(alice) 02 {00|01} h(usdt) borsh(usdt)` (two hashes each), and the slot key is `F0 03 00 h(alice) borsh(alice) 02 02 u32` (one). The spec's existing 59 B vs 43 B for the value key is the same 16. The per-map header key (one hash) is per map, not per entry, so it is not in the 40.

### T6: `default = expr` is const

- The derive emits the default as a `const` item in the field's decode: `|| { const __KEY_DEFAULT: T = expr; __KEY_DEFAULT }`, spanned at `expr`. A non-const expression is one error at the expression: `error[E0015]: cannot call non-const function near_sdk::env::predecessor_account_id in constants` (checked by hand; the doctest checks the code). `default = true` on the Exchange `Market` is unchanged.
- Tests: a `compile_fail,E0015` doctest (`default = env::predecessor_account_id()`) and a passing doctest (`default = DEFAULT_FEE` and `default = true`), both in the `state` module docs. The crate has no trybuild setup, so no `.stderr` snapshot.
- **What broke, and what that means.** The D2 tests used `default = "treasury.near".parse().unwrap()`, `default = "owner.near".parse().unwrap()` and `default = vec![1, 2]`. None is const, and `AccountId` has no const constructor at all (it owns a `Box<str>`; `AccountIdRef::new_or_panic` is const but yields `&AccountIdRef`). They now use `String::new()`, a `const DEFAULT_TIERS: [u8; 2]`, and a `u16`. So under T6 a field of type `AccountId` (or any heap type with a non-empty default) added after the first write can only be `Option<T>`, which D2 said is never forced. Flagged in the spec's Open questions row. A fn-path escape hatch (`default_fn = path`) would answer it; not built, as the decision says.
- No cost change: the const is materialised only when the tag is missing, as the closure was.

### T7: positive value bound, `#[key(n, legacy)]`

- **The trait** is `state::Value` (`near-sdk/src/state/value.rs`). The blanket `impl<T: BorshSerialize + BorshDeserialize> Value for T` is gone, and so are the same Borsh blankets on `Leaf`, `Removable`, `ViewField` and `ViewFieldMut`. All five are now blanket over a new public marker, **`BorshLeaf: BorshSerialize + BorshDeserialize + 'static`**, "an opaque leaf stored as its Borsh bytes". That is an explicit opt-in, not a Borsh blanket. It is implemented for `u8`..`u128`, `i8`..`i128`, `bool`, `()`, `String`, `AccountId`, `NearToken`, `Gas`, `PublicKey`, `json_types::{U64, U128, I64, I128, Base64VecU8}`, and `Vec<T>`, `Option<T>`, `[T; N]`, `BTreeMap<K, V>`, `BTreeSet<T>`, `(A, B)`, `(A, B, C)`, `(A, B, C, D)` of leaves. The marker keeps the change small: every byte encoding is what the Borsh blanket produced, and no op-log or byte count moved. (The coherence worry, a marker blanket overlapping the derive's `impl Value for Record`, does not arise: it is the same rule the Borsh blanket relied on, and the integration tests, ft-v2 and the wasm32 crate all compile.)
- **Opt-in for user types: a separate `#[derive(Leaf)]`**, from `near_sdk::state` (a proc-macro derive in `near-sdk-macros`, re-exported beside the `Leaf` trait like serde's `Serialize`). It emits `impl BorshLeaf for T {}`. I chose it over reusing `#[near(storage)]`, which makes a tagged record with a `#[key]` on every field: `Order`, `Metadata`, `Vote`, `TokenMetadata` and the FT/NFT metadata stay Borsh-encoded with no tags, so the record derive would change their bytes. The test value types now carry `#[near(serializers = [borsh, ..])] #[derive(near_sdk::state::Leaf)]`; none of them is in the printed Exchange block, so the listing did not change for T7.
- **`StorageKey` now requires `BorshLeaf`**, since map keys are stored as values in key slots. Every existing key type already qualifies.
- **Chosen where the decision is silent.** `Option<T>` is a value for leaf `T` only. `Option<Record>` would overlap the marker blanket, and a record is not self-delimiting inside a tuple, so tuples are over leaves too. Neither was possible before (records were never Borsh), so nothing regresses. `BTreeMap`/`BTreeSet`/`near_sdk` types were added because they are common opaque values; `HashMap` was not.
- **`#[key(n, legacy)]`** is new in the derive. It is root only: elsewhere it is a derive error ("only for fields of a `contract_state(layout = v2)` root"). It implies `required`, is exclusive with `default`, and is encoded and decoded with plain Borsh through `__private::{encode_legacy_field, legacy_field}`, outside `Value`, with no binding and no subpath. The bytes are identical to before, which `legacy_root_field_is_its_borsh_bytes_and_required` checks. The derive also rejects a v1 collection *name* at the root without `legacy` ("needs `#[key(n, legacy)]`"). Converted: ft-v2 `v2.rs` tags 1 and 2 (`FungibleToken`, `LazyOption`), `v3.rs` tag 2 (`LazyOption`), the ft-v2 `StatusV2` test root, `state_v2_migrate.rs` (`notes`), and the Exchange `legacy` module. `Migrating<V1, V2>` stays a v2 `Value` (`required`, not `legacy`).
- **The error.** Both `Value` and `BorshLeaf` have the same `on_unimplemented`: "`T` is not a storage v2 value", with two notes (what is a value and the `#[derive(Leaf)]` opt-in; v1 collections only as `#[key(n, legacy)]` at the root, and "check for a `use near_sdk::store::..` shadowing a glob import"). It is noisy: the shadowed-`LookupMap` record reports 14 errors, 10 distinct diagnostics: 9 "is not a storage v2 value" (one spanned at the field type) and 1 "has no `Default`". When the bad type is a map's value (`Map<u64, Order>` without the derive), the message names the `Map`, not `Order`; the note still says what to do. The syntactic v1-name checks stay, because they fire first with a shorter message for qualified paths and catch the root case.
- **What no longer compiles, and where it went:**
  - ft-v2 `tests/nested_trap.rs` is deleted. Its `FungibleToken`-in-a-record case is a `compile_fail,E0277` doctest in `ft-v2/src/lib.rs`, next to a passing doctest of the same type as a `legacy` root field. Its `mod shadowed` case (`use near_sdk::store::LookupMap;` beside `use near_sdk::state::*;`) is a `compile_fail,E0277` doctest on `state::Value`. I checked both by hand: the `FungibleToken` case gives 9 errors, all "`FungibleToken` is not a storage v2 value"; the shadowed import gives the 14 above.
  - Exchange `mod legacy`: the `User { notes: Notes }` record (a `store::LookupMap` behind a type alias, past the name check) and its two tests, `v1_leaf_nested_in_a_cached_record_writes_late_not_lost` and `removing_a_record_with_a_nested_v1_field_orphans_its_entries`, are gone; the alias trick is the second `compile_fail` doctest on `Value` (a Borsh struct holding a `collections::LookupMap`). New: `legacy_root_field_is_its_borsh_bytes_and_required`. Exchange: 35 → 34.
- **Not done.** Nothing stops `#[derive(Leaf)]` or a hand-written `impl BorshLeaf` on a struct that holds a v1 collection; the derive cannot see inside the type. The docs say not to. No trybuild `.stderr` snapshot of the error text.
- Superseded: the "Claim 4" findings above (a nested `FungibleToken` compiles, the shadowed import silently compiles as v1) and the `Removable` caveat about v1 collections being blanket-`Removable`.

### Review fixes to the D9 flag key (`codex-d9-review.md`)

**F1, reinsertion after a blind delete kept both keys (fixed).** Receipt 1 leaves a record mid-clear at its flag key. In receipt 2, `discard(k)` deletes without reading, so the slot's original state stays `Unknown`. Then `try_insert(k, v)` or `entry(k).or_default()` `put`s a new record, which turned the slot from Deleted to Dirty. The flush wrote the record key and, not knowing the flag key held anything, left it. Both keys stayed, and removing the replacement later brought the old record back from the flag key. Fix: a slot remembers `blind_delete` (a delete of a flaggable entry with `Unknown` original state), and a `put` does not clear it. At the flush, a written slot with that bit treats both keys as possibly occupied: it writes the key it belongs at and removes the other one. Cost: only that sequence pays, 1 remove (the flag key) on top of the write; a fresh `try_insert` with no delete before it pays nothing new. Test: `reinsert_after_blind_discard_of_a_clearing_record_drops_its_flag_key`, across three receipts each, for `LookupMap::try_insert`, `LookupMap::entry().or_default()` and `Map::discard` + `Map::try_insert`. The `LookupMap::try_insert` and `Map` cases fail without the fix (checked by disabling it); the `entry` case goes through the same `put`.

**F2, records with the old in-record `00 00` flag (documented, not guarded).** `get`, `get_mut` and `entry` reject such a record with `TagZero`, but `at_mut` only checks that the record key exists, so it permits writes below a record an earlier build was halfway through clearing. The round-2 note's "fails closed" was wrong for `at_mut`. A cheap guard does not exist: `at_mut`'s whole point is not reading the record, and the old flag is inside the bytes. A guard would cost the read the flag key was introduced to remove. So the notes and the spec now say storage written by earlier prototype builds must be reset. Test: `hazard_old_in_record_flag_passes_at_mut_but_fails_get` pins both halves (a write below via `at_mut` lands; `get` panics with `TagZero`).

**F3, relocation re-encoded the record (fixed).** The first partial clear moved the record to its flag key by re-encoding the decoded value. That materialises `default = ..` fields missing from the stored bytes (an old `Market` gained `04 01 01`, 3 bytes), so the round-2 claims "its bytes, unchanged" and "one more staked key byte" were false in general. Fix: `set_clearing` on a record not modified in this call marks the slot `verbatim`, and the flush writes the stored bytes to the flag key unchanged. A `put` or `delete` clears the mark. A record changed earlier in the same call (via `get_mut` before `remove_step`) still moves as its new encoding, which is the change the caller made. The claims are now true as stated, with that qualifier in the spec. No op-count change. Tests: `clearing_moves_the_original_bytes_to_the_flag_key` (exchange: a market seeded without tag 4 keeps its exact 28 bytes at the flag key; it failed before the fix with `04 01 01` appended), and `replaced_then_flagged_record_relocates_the_new_value` (lib: `remove_step`, then `replace`, then `remove_step` again in one call relocates the replacement; it fails if the resets are removed). T6 does not change this: a const default still materialises on any ordinary rewrite (`get_mut` that changes the record); only the relocation is verbatim.

**F4, the first partial clear flushed out of key order (fixed in code; the guarantee text is unchanged).** `flush_ops` for a record starting to clear emitted `Write(record_key ++ 00)` and then `Remove(record_key)`, a descending pair against "dirty keys once each in key order". It now emits the remove first. A record leaving the flag key (`replace`) already emitted `Write(record_key)` then `Remove(flag_key)`, and a blind delete `Remove(record_key)` then `Remove(flag_key)`, both ascending. Same op counts. Test: `partial_clear_flushes_in_key_order` (exchange) asserts that every write and remove of the first partial clear, root excluded, is in non-descending key order, and that the record key comes before its flag key. It failed before the fix.

**`cache::flag_key` bypassed `Path::checked` (fixed).** Round 2 said the flag key's length "`Path::checked` asserts at runtime too". That was true of `Path::flag`, which only tests use, but the cache builds flag keys from raw key bytes, and that path had no check. Both now go through one helper, `key::flag_key_bytes`, which asserts the 2,048-byte limit. In practice the derive's schema sum already rules out an overlong flag key: a record that owns collections always has a longer child key. So the runtime check is a backstop, and now it really exists. Test: `flag_key_bytes_are_length_checked` (2,047-byte record key → 2,048-byte flag key; a 2,048-byte record key panics).

### Op-count changes this round, in one place

- **T1:** none asserted. DAO `add_member` and NFT `nft_mint` each drop 1 `storage_has_key` (the `contains_key` before the insert is gone); no test pins either. `LookupMap::try_insert` of a leaf is 1 check + 1 write where a blind `insert` is 1 write; that is the price of first-write-wins and is new API, not a change.
- **T4:** the call that empties a map writes its header instead of removing it (same count, a write for a remove), and the header stays staked: key + 12 + 40 bytes per emptied map until its owning record is removed. DAO `finalize` (33,0,44) → (33,7,37); `remove_member` (29,3,40) → (29,8,36). Lib: emptying a map or group (3,0,4) → (3,1,3). Headers of maps that had a removal are 12 bytes, not 4.
- **F1:** a reinsert after a blind delete of a flaggable record pays 1 more remove (the flag key), only in that sequence.
- Checked and unchanged: clearing an empty map that never had a header (`remove_step` or `remove` of a record with an empty `balances`, `clear()` on an empty map) costs no header op, because the header read already knows it is absent (`clearing_an_empty_map_without_a_header_writes_nothing_for_it`: (2,0,1), (2,0,1), (1,0,0)).
- **T2, T3, T5, T6, T7, F2, F3, F4, flag-key check:** no op-count change. F4 changes only the order of two ops.

### Deliberately not done

- T8 (generation-numbered handles / Miri aliasing) waits on its audit; T9 to T11 are doc-only and belong to the Claude Doc sync. The Claude Doc is not touched.
- No serde for `Removal`. No `Set::revision()` (a `Set` page carries it). No `Option<Record>` and no tuples of records (T7 notes).
- No fn-path default (`default_fn = path`); a later-added `AccountId` field is `Option<AccountId>` under T6.
- No guard for old in-record `00 00` flags in `at_mut` (F2): it would cost the read the flag key exists to avoid.
- No check that a `#[derive(Leaf)]` type holds no v1 collection, and no trybuild snapshots of the new errors. `#[derive(Leaf)]` adds no bounds on generic parameters, so a generic type that opts in writes its own `where T: BorshLeaf`.
- The near-contract-sim runs (`proto-migration-sim`, `proto-gas`) were not rerun. The T4 header change alters the bytes of every map header after its first removal, so the sim's gas figures for removals could move by the 8 extra header bytes; the T7 and T1 changes do not touch encodings.

### Test counts (end of round 3)

- Lib: 580 (573 + `clearing_an_empty_map_without_a_header_writes_nothing_for_it`, `try_insert_on_leaf_values_keeps_the_first_write`, `page_revision_changes_only_across_a_removal`, `reinsert_after_blind_discard_of_a_clearing_record_drops_its_flag_key`, `hazard_old_in_record_flag_passes_at_mut_but_fails_get`, `replaced_then_flagged_record_relocates_the_new_value`, `flag_key_bytes_are_length_checked`; two renamed).
- Doctests: 346 merged (344 + the T6 const example + the T7 `#[derive(Leaf)]` example) plus 4 standalone `compile_fail` (D6 `insert`, T6 non-const default, T7 shadowed `store::LookupMap`, T7 Borsh struct holding a `collections::LookupMap`).
- `state_v2_exchange`: 36 (35 − 2 nested-v1 tests + `legacy_root_field_is_its_borsh_bytes_and_required`, `clearing_moves_the_original_bytes_to_the_flag_key`, `partial_clear_flushes_in_key_order`). `state_v2_ft`: 16. `state_v2_nft`: 17. `state_v2_dao`: 15. `state_v2_migrate`: 5.
- ft-v2 example: 16 migration; `nested_trap.rs` removed; 2 doctests (the `FungibleToken`-in-a-record `compile_fail` and the `legacy` root that compiles).
- near-sdk-macros: 49.
- `proto-wasm-check`: wasm32 check passes, no warnings.
- `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reasons as before.

## Audit round (Oct 17, Codex claims)

Four claims from the Codex design review (`/tmp/storage-v2-review/codex-design-review.md`, A1, A2,
A4, B) checked with executable tests on `proto/storage-v2-audit`, from `1661f6b5` (`77f63c76` plus
the verify skeleton, which touches none of `cache`, `collections`, `record`, `migrating`). Behaviour
is pinned, not fixed.

Miri (nightly 2026-10-15) runs the UB traces. They are `#[cfg_attr(not(miri), ignore)]`, so plain
`cargo test` skips them:

```sh
NEAR_SANDBOX_BIN_PATH=/usr/bin/true MIRIFLAGS="-Zmiri-ignore-leaks [-Zmiri-tree-borrows]" \
  cargo +nightly miri test -p near-sdk --lib --features unstable,unit-testing -- state::tests::<name> --exact
```

`-Zmiri-ignore-leaks` is needed because slots are `Box::leak`ed until the flush. Miri stops at the first UB, so run one test at a time.

| Claim | Verdict | Tests (commit) | Observed |
|---|---|---|---|
| A1 `cache::get` makes `&mut` on a shared lookup | **PARTIAL**: UB under Stacked Borrows, accepted under Tree Borrows; the "reallocate" part is refuted | `audit_a1_shared_ref_held_across_second_get_of_same_key`, `audit_a1_iter_refs_then_get_of_same_key`, `audit_a1_shared_ref_held_across_gets_of_other_keys` (`62978ddd`) | The traces compile: `Map::get(&self) -> Option<&V>`, so borrowck allows two overlapping lookups and there is no compile error to show. SB: "tag does not exist in the borrow stack" at the later use of the first `&V` (tests.rs:1306, and :1335 for `iter`). The tag was "invalidated by a Unique function-entry retag" at cache.rs:297, `(*p.as_ptr()).as_any_mut()`. TB: both pass, because nothing is written through the `&mut`. A `&V` held across 199 gets of *other* keys is clean under both models: slots and values are separate boxes, and the `BTreeMap` moves only `NonNull`s. |
| A2 `mem::take` of a collection field: lost writes, no rebind panic | **PARTIAL**: the mechanism is wrong (no write is lost), but the class is real (UAF, orphans, D9 bypass) | `audit_a2_taken_field_handle_and_recreated_handle_stay_coherent`, `audit_a2_ref_from_taken_handle_dangles_after_overwrite_through_recreated_handle` (Miri), `audit_a2_taken_handle_writes_after_parent_remove_orphan_and_resurrect`, `audit_a2_taken_handle_bypasses_remove_some_clearing_guard` (`e0b7486b`) | Collections are `Default` (`Unbound`), so `mem::take(&mut rec.balances)` compiles via `get_mut`, and via `at_mut` too (its fields are `&mut` handles in the arena). The taken handle stays `Bound`, and no panic fires. Both handles route through the same path-keyed cache slots, so each sees the other's writes and the flush writes all of them: **no lost write.** What breaks: (1) a `&V` from the taken handle, then an overwrite through the recreated one (`put` → `drop_value`, cache.rs:160), then a read of the `&V`. Miri reports use-after-free under SB and TB. (2) After `accounts.remove(&alice)`, the taken handle is not detached (only the left-behind one is), so `taken.insert` writes 4 orphan keys (entry, index, slot, header) under the removed path. A later `insert_new(alice, ..)` inherits them (`len 1`, `z = 9`). (3) After `remove_some(&alice, 0)` flags the record, `taken.insert` below it is accepted. The D9 guard checks the record path only in `get_mut`/`entry`/`at_mut`. Later `remove_some` calls still clear the added entry. Nothing blocks the escape at the type level: invariant 3 in cache.rs says single-handle-per-key is not type-enforced, and `__bound_at` is a public (doc-hidden) constructor. |
| A4 `new_side()`/`old_side()` break the one-side invariant | **CONFIRMED** | `audit_a4_new_side_write_is_overwritten_by_migrate_batch`, `audit_a4_remove_after_new_side_write_resurrects_the_old_value`, `audit_a4_old_side_write_overwrites_the_new_value_at_the_next_batch` (`90fcc10c`, `state_v2_migrate`) | (1) `new_side().insert(tok0, buyer)` while v1 holds tok0: reads return `buyer` until the batch, then `owner(0)`, a lost update. (2) Same setup, then `remove(tok1)` returns `buyer`, and the next `get(tok1)` is `owner(1)`, resurrected through the fallback; the batch then makes it permanent. (3) Mirror: a stale value put back through `old_side()` after the key moved replaces the new value at the next batch. `migrate_batch`'s doc relies on exactly the invariant these accessors let a caller break. |
| B gas and storage arithmetic | **CONFIRMED**, with one refinement | none (arithmetic from GAS_MEASUREMENTS.md, the spec and key.rs) | See below. |

### B, the arithmetic

- **(i) 138 Ggas "nested read".** It is `balance_of` − `noop_view` = 1.288 − 1.150 Tgas, a workload delta. By column: base 112.7 − 56.4 = 56.3, per-byte 2.1 − 0.1 = 2.0, sha256 9.6, wasm 44.5 − 20.4 = 24.1, other 946.4 − 899.8 = 46.6; total 138.6. The 46.6 of "other" is mostly the JSON argument and return value of the method, so it is a property of the method, not of the read. The read itself is ≈ 56.3 + 2.0 + 9.6 ≈ 68 Ggas plus ≈ 24 Ggas of wasm decode. The 89 Ggas "root read" is likewise `noop_view` − `ping`. Table 4 has no trie-node or load column; its residual equals the 172.9 Ggas load exactly, so reads there show 0 trie-node gas, consistent with flat storage.
- **(ii) 0.527 Tgas `Map` premium** (100k keys + 100k accounts) = 1.855 − 1.328 Tgas. By column: storage base 425.9 − 120.6 = 305.3 (2 reads × 56.4 + 3 writes × 64.2 = 112.7 + 192.6), trie 148.2 − 36.5 = 111.7 (49 nodes), per-byte 6.5, sha256 10.3, wasm 36.7, other 57.1, load 0; total 527.6. So 0.305 + 0.112 + ~0.110 holds. The refinement: the ~0.110 remainder is mostly "other" (57, register/memory/exec) and wasm (37), and only 10 is hashing. The file's own one-line breakdown (line 97) omits the 6.5 Ggas per-byte term and sums to 522.
- **Hash width, bytes.** key.rs puts one hash per `entry()` hop: `kind ++ h(k) ++ borsh(k)`. Slots are `kind ++ u32 LE` with no hash. Per `accounts[alice].balances[usdt]` entry: value key 59 B (2 hashes), index key 59 B (2 hashes), slot key 39 B (1 hash, the outer one). At 8 bytes instead of 16 that saves 16 + 16 + 8 = **40 B per entry**: staked bytes go from 115/103/87 = 305 to 99/87/79 = 265 (−13%). The header (35 B key, 1 hash) saves 8 B once per map, and the record key 8 B once per account. Cost: 40 B × 10⁶ entries × 10¹⁹ yoctoNEAR/B = 4 × 10²⁶ yocto = **400 NEAR per million entries**. For `LookupMap` (the value key only) it is 16 B, 160 NEAR per million. The GAS_MEASUREMENTS verdict "doesn't matter; keep 16" is a gas verdict only; the storage-stake side is not weighed anywhere.

### Smallest fixes (not implemented)

- **A1.** In `cache::get`, never form `&mut` for a lookup. Type-check through a shared borrow (`(&*p.as_ptr() as &dyn Any).type_id() == TypeId::of::<V>()`, or an `as_any(&self)` method) and return `p.cast::<V>()`. Every `&V` and `&mut V` then derives from the slot's raw pointer. `get_mut`'s `m(p)` stays sound, since it is still raw-derived. Do not use `NonNull::from(downcast_ref())`: that would make `get_mut` write through a shared-derived pointer.
- **A2.** The escape cannot be closed at the type level while records hold owned handles: `mem::replace` and `mem::swap` work even without `Default`. Two runtime fixes, in order:
  - Defer frees: `drop_value` in `put`/`delete` moves the old box to a per-call graveyard freed at the flush. That closes the UAF with no API change. A `&V` vs `&mut V` overlap across two handles is still an aliasing violation, though not a dangling read.
  - The cache keeps the set of record paths removed or flagged clearing in this call, and `put`/`touch` under such a prefix panics. That closes the orphan writes and the D9 bypass within a call. An escaped handle cannot outlive the call except by deliberate statics.
  - Longer term: `&mut Record` should not expose collection fields as owned values (`get_mut` returns a view, as `at_mut` does). Also make `__bound_at` crate-private, or rename it to say it is unsafe.
- **A4.** `new_side()`/`old_side()` return `&V2`/`&V1`, or are removed. As defence in depth, `migrate_batch` skips keys already on the new side (1 extra read per moved key), and `remove` deletes from both sides (1 extra old-side remove).

### Unsafe inventory

The cache uses `unsafe` throughout, by design. cache.rs has 23 `unsafe` blocks, every slot or value deref (lines 146, 160, 198, 230, 235, 279, 287, 297, 314, 340, 356, 366, 380, 392, 401, 409, 414, 425, 435, 454, 474, 495, 515). collections.rs has 37 `unsafe` sites: the `r`/`m`/`arena_mut` helpers and their call sites. root.rs has one `unsafe fn get_record`. record.rs, migrating.rs, key.rs and value.rs have none.

## API round 4 (Oct 17)

Fixes for the audit findings A1, A2 and A4 (`/tmp/storage-v2-review/audit-results.md`), plus three small reversals of round 3, on `proto/storage-v2-d9`. One commit per item. The audit branch `proto/storage-v2-audit` (689c9ec4) was merged first. Its two conflicts were end-of-file appends (this file, `state/tests.rs`), and both sides are kept. Its tests were ported to round-3 names (`try_insert`, `remove_step -> Removal`) and to the D9 flag key: a clearing record lives at `rec ++ 00`, which `audit_keys_below` counts. The merge also brings the verify skeleton (`state/model.rs`, `state/verify/`, `bolero`, `cfg(kani)`), untouched here.

### A1: no `&mut` on the read path

- `Erased::as_any_mut` is gone; `Erased::as_any(&self)` replaces it. `cache::get` checks `is::<V>()` through a shared borrow and returns `p.cast::<V>()`, keeping the box's provenance. `cache::put` returns `p.cast()` too. Invariant 1 in `cache.rs` now says the read path forms no `&mut`.
- The three `audit_a1_*` tests were `#[cfg_attr(not(miri), ignore)]`, not `cfg(miri)`-only. They are now plain `#[test]`: safe code that passes in `cargo test`, where it checks values only. The verdict is Miri's, and the header comment gives the command.
- Miri (nightly, `-Zmiri-ignore-leaks`): 3/3 pass under Stacked Borrows (the same-key trace was UB at `cache.rs:297` before) and 3/3 under Tree Borrows. Miri's clock is virtual under isolation, so equal "finished in" times across models are expected; the wall times differ.

### A2: escaped handles (`mem::take` on a record's collection field)

- **(a) Use-after-free, fixed for overwrites and value-less deletes.** `cache::put` no longer frees the old value. It moves the box to a per-call `graveyard` that `free_slots` frees after the slots at the flush. The cache API is split: `delete` / `delete_entry` return nothing and bury the value; `take` / `take_entry` return it and are used only where the API hands the value back (`LookupMap::remove`/`replace`, `Map::remove`, `Vector::pop`/`swap_remove`). `Map::discard` now buries rather than drops. Invariant 2 in `cache.rs` is rewritten to match.
- **Hole left on purpose:** a removal that *returns* the value must move it out of its box, so a `&V` from an escaped second handle still dangles after `remove`/`replace`/`pop`/`swap_remove` through the other handle. Closing it would mean re-decoding (or cloning) every returned value, compute on every remove, for a case only `mem::take` reaches.
- Memory: values overwritten in a call stay allocated until its flush (a key overwritten 1,000 times in one call holds 1,000 boxes). Gas bounds the count.
- **(b) Writes through a stale handle panic: generation numbers** (persona item 6), chosen over a per-call set of removed prefixes. A prefix set cannot tell an escaped handle from the fresh handles of a record re-inserted at the same key in the same call (`remove` then `try_insert`, or `replace`, then a write below). Either it refuses that legitimate sequence, or it re-allows the prefix and with it the escaped handle.
  - `Binding::Bound(Path, u64)` carries the epoch the handle was last known live at, stamped by `bind_slot` and `__bound_at`. A manual `Debug` keeps the output `Map(Bound(..))`.
  - A thread-local `EPOCH: Cell<u64>` is never reset, not even by `install_host`. `Cache::record_gone(key)` bumps it and logs `(epoch, key)` for the call. It is called by `delete_impl` when `V::MAX_SUBPATH > 0` (so `Map`/`LookupMap` remove/discard/replace/clear of records, `Lazy<Record>::remove` and nested clears all count) and by `set_clearing` (`remove_step` returning `Pending`). The log is cleared at the flush.
  - Every `&mut self` method of `LookupMap`, `Map`, `Vector`, `Lazy` and `GroupedMap::group_mut` (28 sites) starts with `self.path.live()`. `Set`/`LookupSet` delegate. `cache::check_live` returns at once if the stamp equals the epoch. Otherwise it scans the log backwards, only entries newer than the stamp, for a key that is a strict byte prefix of the handle's path. A hit panics ("belongs to a record that was removed or started `remove_step` clearing in this call after this handle was made"); a miss moves the stamp to now.
  - **Per-op cost:** one thread-local load and one `u64` compare per write while no removal happened since the handle last checked. After a removal, one scan of the new log entries per handle, so each entry is scanned at most once per handle. Prefix soundness rests on the key layout: map keys are prefix-free behind their hash, and tags are prefix-free LEB128. No host-call change.
  - Reads are not checked: `get`/`at` of a clearing record are allowed, and a cached record decoded before `remove_step` in the same call still reads its maps.
  - Scope: removals in the current call. A handle surviving into a later call is impossible in a contract (the root is decoded per call) and is not covered. `__bound_at` (doc-hidden, public) mints fresh handles and can bypass the check.
- Tests:
  - `audit_a2_ref_from_taken_handle_dangles_after_overwrite_through_recreated_handle` is no longer ignored and asserts the `&V` still reads 1. New: `audit_a2_ref_from_taken_handle_survives_discard_through_recreated_handle`.
  - `audit_a2_taken_handle_writes_after_parent_remove_orphan_and_resurrect` and `audit_a2_taken_handle_bypasses_remove_some_clearing_guard` keep their names but now assert the panic with `catch_unwind`. The first also checks `get_mut`, `entry`, `remove` and `clear`. Both then assert nothing is orphaned: 0 keys below the removed record and a recreated alice is empty; and after clearing, no `late` entry and 0 keys left.
  - New: `audit_a2_remove_then_reinsert_then_write_below_in_one_call` (also through `remove_step` → `Done` → `entry().or_default()`) and `audit_a2_unrelated_removal_does_not_refuse_other_handles`.
  - Miri: 7/7 under both models.

### A4: `Migrating` sides

- `new_side(&self) -> &V2`, `old_side(&self) -> &V1`. The migrate flow needs no mutable accessor, but the A4 tests must break the rule on purpose, and v2 operations `Migrating` does not forward need a way in. So the mutable ones exist under names that say what they do: `new_side_mut_unchecked()` and `old_side_mut_unchecked()`. Their docs say they bypass the one-side rule and what still holds if they break it.
- `migrate_batch` moves through a new `MigrationTarget::target_try_insert`: an old entry whose key is already on the new side is dropped, not moved. It costs nothing for a `Map` target (`try_insert` = `insert`: the index read decides) and +1 `storage_has_key` per entry for a `LookupMap` target.
- `remove` deletes from both sides. A key on the old side costs what it did. A key on the new side pays 1 old-side remove more: `TreeMap`'s `contains_key` read (no tree walk), 1 `storage_remove` for a v1 `LookupMap`, a deferred `set(None)` for `store::LookupMap`.
- The three `audit_a4_*` tests keep their names and now assert the fixed behaviour: the newer value survives the batch (`new_side().len() == N`), no resurrection after `remove`, and a stale old-side write is dropped and gone from the old side. They set up the broken state through the `*_unchecked` accessors.

### T1, partly reversed

`Map::insert`'s bool is not `#[must_use]` (the `HashMap`/`HashSet` precedent); the rustdoc says so. `try_insert`'s `Result` is unchanged. The Exchange `set_balance` line is again `self.accounts.at_mut(&id).expect("no such account").balances.insert(token, amount.0);` in the spec, `proto-wasm-check` and `state_v2_exchange.rs`. Every listing line in the spec matches both copies; the test file's `use` line differs, as before. The 43 `let _ =` round 3 added at other test sites are harmless and stay.

### T6, loosened

- `default = expr` stays const-only. New: `#[key(n, default_fn = path)]`. The derive emits `let __key_default_fn: fn() -> T = path;`, spanned at the path, then calls it in the decode closure. So a wrong signature is E0308 at the path, and a closure or call is a derive error ("`default_fn` takes a path to a `fn() -> T`, not an expression"); both were checked by hand. `default_fn` is exclusive with `default`, `required` and `legacy`.
- Documented loudly in the `state` rustdoc and the spec: called on every decode where the tag is absent, until some other change rewrites the record; must be pure. A `get_mut`/`entry` writes the tag at the flush, because the re-encoding differs.
- The D2 tests are back to D2's types: `Defaults { treasury: AccountId (default_fn = default_treasury), tiers: Vec<u8> (default_fn = default_tiers) }` and `SlimV2Default { owner: AccountId (default_fn = default_owner) }`, with the original assertions. The `DEFAULT_TIERS` const is gone.
- **Spec.** There was no separate Open questions row about forced `Option`: round 3 had appended that caveat to the D2 "Fields without `Default`" row. I removed the caveat and named `default_fn` in the row, so it again reads "never a forced `Option`". The "Two new attributes" paragraph, the Permanent layout bullet and the derive-error bullet now mention `default_fn`.
- New doctests: a passing `default_fn` example with an `AccountId`, and a `compile_fail` closure (no error code: a derive error has none).

### T2 wording

The spec's rule sentence now says `at*` is "path arithmetic plus at most two existence checks, never decodes". `at` makes none; `at_mut` makes one on a present record and two on an absent record that owns collections.

### Op-count changes this round

- One: `Migrating::remove` of a key already on the new side, with a v1 `TreeMap` old side, goes from (4,1,3) to (5,1,3), +1 read. New test: `remove_of_a_moved_key_also_removes_on_the_old_side`, which replays the round-3 path by hand for the "before".
- `migrate_batch` into a `LookupMap` target would pay +1 `storage_has_key` per entry; no test or example uses one.
- Unchanged: every lib, Exchange, FT, NFT, DAO, migrate and ft-v2 count. A1, A2, T1, T2 and T6 add no host calls; A2 adds the in-memory epoch compare.

### Deliberately not done

- No proxy view types for `get_mut`/fields (the audit's longer-term fix). No duplicate-handle detection without a removal: two live handles to one path still coexist, coherent through the shared cache.
- The value-returning-removal UAF hole (A2(a) above).
- Cross-call stale handles; `__bound_at` as a bypass.
- Reverting round 3's other `let _ =` sites; a trybuild snapshot of the `default_fn` errors.
- The Claude Doc; the near-contract-sim gas runs (no encoding changed).

### Test counts (end of round 4)

- Lib: 591 passed, 1 ignored (`promise::tests::test_yielded_promise_can_chain_then`, pre-existing). That is 580 + 7 merged audit tests in their ported form (3 A1, 4 A2), all un-ignored + 4 new A2 tests.
- Doctests: 347 merged (+ the `default_fn` example), 7 ignored; 5 standalone `compile_fail` (+ the `default_fn` closure).
- `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15, `state_v2_migrate` 9 (5 + 3 audit A4 + `remove_of_a_moved_key_also_removes_on_the_old_side`).
- ft-v2 example: 16 migration + 2 doctests. near-sdk-macros: 49 (+1 doctest, 4 ignored).
- `proto-wasm-check`: wasm32 check passes, no warnings.
- Miri (`-Zmiri-ignore-leaks`), 10 audit lib tests: 10/10 under Stacked Borrows, 10/10 under Tree Borrows.
- `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reasons as before.


## Soundness round (Oct 17)

Goal: make each of the seven guarantees and each named bug class machine-checked with a stated bound. Everything lives in `near-sdk/src/state/verify/` (lib harnesses), `near-sdk/src/state/model.rs` (the reference model), and `near-sdk/tests/state_v2_verify.rs` (through the real `#[near(storage)]` derive).

Engines:

- bolero's random engine, under `cargo test`.
- Kani 0.68 on the host, run with `NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo kani --tests --lib -p near-sdk --features unstable --harness <name>`.
- libFuzzer: **not run, toolbox unavailable** (cargo-bolero is not installed and Docker was not running).

### Reference model (Part A)

`model.rs` is an independent implementation:

- its own key encoder (`spec_key`);
- an abstract state, `MState`, which `concretize()` turns into store bytes;
- a cache model covering slot states Unknown/Absent/PresentUnread/Present and modes Clean/Touched/Dirty/Deleted.

`verify/diff.rs` drives the real types and the model over the same generated history: up to 60 ops, including mid-call flushes, views, and retyped (schema-evolved) calls. After every call it compares:

- results;
- the multiset of reads;
- the ordered list of writes and removes;
- the store bytes.

It also asserts that no write happens before the flush. The oracle was validated by mutation:

- dropping the moved-key index write fails Part A;
- truncating field tags fails G1;
- an extra read fails G2.

### Harness table

"Diff" means the harness is differential against the reference model or another oracle. "Inv" means an invariant checked on the real implementation alone. Random-engine iteration counts are the defaults; `BOLERO_RANDOM_ITERATIONS` overrides them.

| Item | Harness | Checks | Engine, bound | Result | Kind |
|---|---|---|---|---|---|
| Part A | `diff::differential_against_reference_model` (+ fixed sequences) | results, read multiset, ordered writes/removes, store bytes per call; no write before the flush | random, 2,000 histories of up to 60 ops over 5 keys | pass | diff |
| G1 Addressable | `g1_field_tags_are_prefix_free_over_u32` | `root ++ LEB128(tag)` decodes back, and two tags never prefix each other | **Kani, every pair of u32 tags: VERIFIED (6 s)**; random 20,000 | pass | inv |
| G1 | `g1_addressable_distinct_paths_distinct_keys` | two distinct paths give distinct keys, with free symbolic placement hashes | Kani: depth 4 did not complete in 40 min; depth 2 did not complete in 30 min (CPU shared with mutants). Not verified. Random: depth 4, real sha256, varint-edge tags, 20,000 pairs | random pass | inv |
| G1 | `g1_production_keys_equal_spec_keys`, `g1_long_paths_up_to_the_protocol_limit`, `g1_key_of_exactly_2048_bytes...`, `g1_identical_hashes_still_give_distinct_keys`, `g1_field_tag_varint_transitions`, `g1_nested_get_reads_only_its_own_key` | the production encoder equals the independent spec encoder (F1); 2,048-byte limit accepted / 2,049 panics; zero reads to compute a key | random 20,000 / 5,000; fixed cases | pass | diff + inv |
| G1/M4 (C7) | `c7_admitted_v1_prefixes_never_alias_v2_keys` | an admitted v1 prefix never aliases a v2 key (forged-tail mode makes the alias reachable); rejections carry the right reason | **Kani: any root byte, prefixes 0..=3 bytes, suffix 0..=4 bytes or forged tail, any nonzero u32 tag, any kind, free 16-byte hash, 0..=2-byte key: VERIFIED (152 s)**. (The first Kani run found a harness bug, symbolic tag 0, now excluded.) random 100,000 | random pass | inv |
| C7 | `c7_production_v1_keys_vs_production_v2_keys` + derive tests (`m4::*`) | production v1 keys from all 3 hashers vs production v2 paths; an empty prefix forges any v2 key; the guard fires on write (no write made) and on load, for every v1 map type | random 20,000; fixed | pass | inv |
| G2 Local | `g2_local_op_touches_only_its_own_path_and_bookkeeping` | every host call is in the op's allowed set; no other entry's value key; the 100 KiB neighbour never moves | random 3,000 (about 2,000 reach the checks) | pass | inv |
| G3 No debt | `g3_no_debt_counts_depend_only_on_the_ops_own_key`, `g3_no_debt_after_10000_random_operations` | op cost depends only on its own key's facts, after any history | random 5,000; 3 histories of exactly 10,000 ops | pass | diff (formula) |
| G4 One write | `g4_one_write_per_key_per_call_in_key_order_root_last`, `g4_nothing_happens_in_drop`, `g4_root_is_last...` | each key written at most once per call, in key order, root last; views never write; no writes from Drop | random 2,000 each | pass | inv |
| G4 | `g4_cache_flush_writes_each_key_once_in_order_root_last` | the same, on the cache directly with a recording host (also F2 slot states) | random 20,000; **Kani not run**: Kani 0.68 ICEs on any reachable `catch_unwind` | pass | inv |
| G5 Linear iteration | `g5_page_costs_two_reads_plus_two_per_entry`, `g5_page_of_64_on_an_aged_map` | 2+2n reads (keys only: 2+n), exact window, CursorGone in 1 read | random 5,000; maps of 0..=200 after 80 removals | pass | diff (mirror) |
| G6 Permanent layout | `g6_record_encode_then_parse_is_identity` | encode then parse is the identity | Kani not verified: the only bound attempted, 2 fields x 4 bytes, did not finish in 20 min; random 20,000 (4 fields, 16 bytes) | random pass | inv |
| G6 | `g6_record_parse_accepts_only_canonical_encodings` | one encoding per record | Kani not verified: strings of 6 bytes (about 27 min) and 4 bytes (15 and 20 min) did not finish; random 50,000 strings of up to 64 bytes | random pass | inv |
| G6/D2/D4 | `g6_schema_evolution_defaults_required_unknown_reserved` | defaults, `default = expr`, required fails naming the tag, unknown tags byte-preserved through a rewrite, reserved tags dropped | Kani not verified: 4/3/2 tags did not finish (15-30 min); random 20,000 (4 tags x 5 decls) | random pass | diff (oracle) |
| G6/D2 derive | `tests/state_v2_verify.rs` `g6_derive_*`, `d2_derive_required_*` | S1 to S2 through the real derive: added (Default / `default =`), removed, reordered (byte-identical), required ("Missing { tag: 4 }"), reserved; zero writes on decode | random 2,000 (500 for required) | pass | diff |
| G7 Stated cost | `g7_stated_cost_beside_a_100kib_neighbour` (+ 3 pinned) | every op's receipt equals the spec formula, and is identical beside a 1-byte neighbour; `at_mut` measured = (3 reads, 4 sha); sha256 per op measured | random 20,000 | pass (deviations in DOC_DIFFS) | diff (formula) |
| C1 HOT collision | `c1_spliced_keys_never_collide_across_collections`, `c1_baseline_v1_raw_prefix_maps_collide` | adversarial keys spliced from other collections' storage keys, across sibling, lookup and nested maps: distinct keys, own values; the v1 baseline does collide | random 5,000 | pass | inv |
| C2 Orphans | `c2_no_orphans_after_random_histories`, `c2_removed_record_leaves_no_key_under_its_path` | every stored key under the record map belongs to a live record's bookkeeping; remove / discard / remove_some leave nothing behind | random 2,000 each | pass | inv |
| C3 Lost writes | `c3_a_mid_call_flush_anywhere_loses_no_write`, `c3_fixed_shapes...`, `hazard_two_roots...` | inserting a mid-call flush anywhere leaves the final store identical | random 2,000 | pass | metamorphic |
| C4 storage_delta | `c4_storage_delta_matches_storage_usage_and_the_guard_is_sound` (+ 2) | logged `storage_delta` equals the change in `storage_usage`; a quiet guard implies only the root is written; root-scalar gap pinned | random 500 histories, env host | pass | diff (runtime rule) |
| C5 remove_some | `c5_remove_some_isolates_the_record_until_it_is_gone`, `c5_pinned_budget_zero...` | writes below a clearing record panic without effect (same and later calls); exact progress min(n, remaining); calls <= ceil(m/n)+1; flag written only once | random 3,000 | pass | diff (mirror) |
| C6 Paging under churn | `c6_paging_under_churn_is_exact_or_cursor_gone` (+ 2 pinned) | exact window or CursorGone; offset never duplicates; insert-only churn is complete; skips only of moved keys; **cursor duplicates found** | random 5,000 | pass | diff (mirror) |
| C8 = D2 | see G6 rows | | | | |
| F3 Atomicity | `f3_a_failed_call_leaves_no_trace` | a panic at every flush position or in the body, then revert: equals the model without that call | random 2,000 (784 flush and 1,873 body failures injected) | pass | diff |
| A3 Rollback | `a3::hazard_rollback_new_record_inherits_old_children` | pinned hazard | fixed | hazard reproduces | pin |
| A5 Liveness | `a5::hazard_lazy_values_consume_no_clearing_budget`, `..._budget_zero` | pinned hazard with bytes per attempt | fixed | hazard reproduces | pin |

### Doc corrections found (not applied; docs/ untouched)

Measured, and pinned by `verify/cost.rs` `DOC_DIFFS` and `g7_stated_cost`, unless noted:

1. Map/Set remove of the LAST key also skips the last-slot read: 3 reads for remove, 2 for discard, not 4. **[Oct 18: still true. `Map::remove` rustdoc fixed; spec open.]**
2. `LookupMap::discard` / `LookupSet::remove` of an absent key still issue one blind `storage_remove`. **[Oct 18: still true. Rustdoc fixed; spec open.]**
3. `Lazy::remove` issues one blind `storage_remove` even when the Lazy is empty. **[Oct 18: still true. Rustdoc fixed; spec open.]**
4. An unfinished `remove_some` call reads one more key slot than it removes: 2 + n + 1 reads. **[Oct 18: still true for `remove_step` (now record lookups + 1 + n + 1: 1 or 2 record reads, the header, n + 1 slots). Rustdoc fixed; spec open.]**
5. `remove_some` writes the clearing flag only on the FIRST unfinished call; later unfinished calls write nothing. The spec says every such call writes it. **[Oct 18: resolved by round 2: the spec now says the first call that does not finish removes the record key and writes the flag key.]**
6. `at_mut` is 1 record read, with no decode and no `storage_has_key`. The rustdoc on `Map::at_mut` / `LookupMap::at_mut` still says "1 storage_has_key" (stale). **[Oct 18: resolved by round 2: `at_mut` is again 1 `storage_has_key` (2 on a miss), which the rustdoc says.]**
7. `Vector::swap_remove` elides the rewrite of slot i when the moved element's bytes are identical to the removed one's. **[Oct 18: still true. Rustdoc fixed; spec open.]**
8. `page` / `keys_page` with limit 0 return `next = None` even when entries remain. **[Oct 18: still true. `Map::page` rustdoc fixed; spec open.]**
9. sha256 per operation, measured: Map insert new 3, insert existing 2, inner remove/discard 3, at_mut + insert below a record 4, record remove owning 2 items 7. **[Oct 18: still true, counts unchanged after the merge. Spec only; no state rustdoc states a count.]**
10. (C6) Cursor paging can return keys twice when a swap-remove moves the cursor key toward the front. The `Map::page` rustdoc names only the skip. **[Oct 18: still true; T4's `revision` makes it detectable. `Map::page` rustdoc fixed; the spec's Pagination paragraph names only the skip.]**
11. (A5) "each call removes at most n entries" holds for map entries only. Lazy values below the record are removed in the first attempt, and a Lazy-only record goes with n = 0. **[Oct 18: partly resolved: the spec now says Lazy values do not count. Still open in the spec: "`n = 0` clears nothing and sets the flag" is false for a record with only Lazy values or empty maps (it finishes). Rustdoc fixed.]**
12. (C5) `remove_some(k, 0)` on a record that still has entries never makes progress. **[Oct 18: resolved by round 2 in the spec ("`n = 0` clears nothing and sets the flag"); rustdoc now also says it returns `Pending` forever.]**

The `set_balance` comment in the Exchange example is known-stale and was left alone, as instructed.

### Hazards pinned (tests that pass while the hazard exists)

- **A3, rollback subtree ownership** (`a3::hazard_rollback_new_record_inherits_old_children`). Sequence: version B gives a record a child map; a rollback to A removes the record and recreates it; redeploying B shows the new record the old record's children. Missing invariant: a record's subtree belongs to the record *instance*, not its key. Either removal clears every key under the record's path whatever the current schema (a prefix delete), or record paths carry an instance/generation component.
- **A5, Lazy values consume no clearing budget.** With 4 x 64 KiB Lazy fields and 3 map entries, `remove_some(k, 1)` removed (removes, value bytes) per attempt as `[(7, 262166), (7, 6), (12, 24)]`. A record holding only Lazy fields is removed whole by `remove_some(k, 0)`. So bounded clearing is bounded in entries, not in bytes or gas.
- **C3, two roots alive in one call** lose the first root's scalar write. This is not reachable through the `#[near]` wrapper.
- **C4/D5 gap:** a change to a root scalar alone does not trip the `storage_usage()` guard, but its flush still charges storage.
- **C5, `remove_some(k, 0)`** on a record with entries makes no progress. A loop with n = 0 never finishes.
- **C6, cursor paging can return keys twice.** A swap-remove can move the cursor key itself toward the front. The `Map::page` rustdoc names only the skip, so this is a doc correction. Offset paging never duplicates; it only skips moved keys.
- **C7/M4:** a v1 map that is *not* wrapped in `Migrating` (a bare v1 field beside the v2 root) is unchecked. With an empty prefix and a `[u8; 1]` key, it overwrites the v2 root record.
- **G6/D4 tag reuse:** already pinned by `state_v2_exchange::hazard_reusing_a_removed_tag_resurrects_its_stale_bytes`.
- **C2, v1-nested exception:** already pinned by `state_v2_exchange::legacy::removing_a_record_with_a_nested_v1_field_orphans_its_entries`.

### Code changes outside verify/

- `check_v1_prefix(prefix, root) -> Result<(), V1PrefixError>`, the M4 migrate-time guard. It runs in `Migrating`'s `bind`, `decode_value` and `__write_pending`, so an aliasing migrate fails before any write and an aliasing stored layout fails to load. It is conservative: a rejected prefix need not alias in practice.
- `LegacyMap::legacy_prefixes()`, implemented for every v1 map: `collections::{LookupMap, UnorderedMap, TreeMap}` and `store::{LookupMap, IterableMap}`, the latter over Identity-keyed tables only, since hashed keys are 32-byte digests.
- `ToKey::RAW_PREFIX`, a hidden const with a default; small `pub(crate)` prefix accessors.
- `Path::entry_hashed`, so that harnesses can supply a symbolic hash.
- `kani` added to the lib's cfg allowlist and to `check-cfg`; `bolero = "0.13"` added as a dev-dependency.

### Mutation testing

cargo-mutants 27.1.0, run with `--in-diff` (`git diff master...HEAD -- near-sdk/src/state`, i.e. the whole v2 branch, not only this round). verify/, model.rs, tests.rs and testing.rs are excluded. Two passes:

1. A fast pass: lib `verify::` harnesses only, with `BOLERO_RANDOM_ITERATIONS=300` and the fixed 10,000-op test skipped.
2. `--iterate` over the survivors: full random iterations, the whole lib, plus `state_v2_verify`, `state_v2_migrate` and `state_v2_exchange`.

**Core files** (cache, key, record, root, value, migrating; 333 mutants):

| | Count |
|---|---|
| Caught | 200 |
| Timeouts (also caught) | 7 |
| Unviable | 61 |
| Missed | 65 |

Breakdown of the 65 missed:

- **Equivalent, 7:**
  - `write_varint` `|` to `^` (the low byte is below 0x80);
  - two `value.rs` defaults;
  - the 4 mutants of the default `LegacyMap::legacy_prefixes` body (every impl overrides it).
- **Migration paths the state_v2 tests do not assert on, 29.** Some are exercised only by the ft-v2 example workspace, which is outside the test command:
  - `raw_root` / `root_orig` / `parse_orig` (14);
  - the `Migrating` `LookupMap` target and `NewMap::target_remove` (4);
  - `legacy_get` / `legacy_remove` / `legacy_take` / `legacy_is_empty` of the v1 maps (11).
- **Closed after the run, 4:** `collections::LookupMap::legacy_prefixes`, by adding that type to the C7 derive test. Each of the 4 mutants fails it by construction; this was not re-run.
- **Real harness gaps, 25:**
  - `has_pending_writes` (4): the view assertion is never driven negatively;
  - static `MAX_LEN` arithmetic for tuple and `Option` keys (4);
  - the derive's `MAX_SUBPATH` helpers `__private::{max, field_subpath, varint_len}` (7); the runtime 2,048-byte check still backs them up;
  - `Migrating::{contains_key, remove, new_side, bind, detach}` (6), which no test calls;
  - `EnvHost::has_key` (1), `Path::is_empty` (2), the `Path` `Debug` impl (1).

**collections.rs** (331 mutants), after pass 1 and two full-test iterates (the second includes the nested-collection C2 test):

| | Count |
|---|---|
| Caught | 144 |
| Timeouts (also caught) | 5 |
| Unviable | 107 |
| Missed | 75 |

Breakdown of the 75 missed:

- **Unobservable, 2:** the `Display` impl of `CursorGone` and the `Debug` impl of `Lazy`.
- **Nested Lazy / Set / LookupSet / Vector inside a record, about 35.** No reference-model or invariant coverage for:
  - inline `encode_value` / `decode_value`, `detach`, `__write_pending`;
  - `Lazy::{path, __bound_at, get_mut}`;
  - the `At<..>` view accessors.

  The lib fixture nests only a `Map`; the derive tests reach these types but assert only on the final store.
- **Per-attempt budget accounting of nested Vector/Set in `remove_some`** (`clear_children_some` returning true early, or its arithmetic). This is unobservable in the final store, because the finishing call's unbudgeted clear still removes everything, but it breaks the per-attempt bound: the A5 pattern again.
- **API surface no harness calls:**
  - `IndexIter::{size_hint, next_back, nth_back}`;
  - `len` / `is_empty` of Map, Set and Vector;
  - `At<LookupMap>::{get, contains_key}`.
- **`MAX_SUBPATH` arithmetic** (collections.rs:1059, 1533). The runtime 2,048-byte check still backs it up.
- **`assert_insert_new`**: it guards `Lazy::set` of a record over an existing record. Uncovered.
- **`record_at_mut`'s `>` to `>=`.**

`insert_new` on an existing Map/LookupMap record (a separate guard) is now asserted by `c2_insert_new_on_an_existing_record_panics`.

### Not done / skipped

- libFuzzer / cargo-bolero: **not run, toolbox unavailable** (cargo-bolero is not installed; Docker was not running). Every bolero harness ran under the random engine only.
- Kani, harnesses that did not finish under the stated time caps (Kani 0.68, CBMC 6.11, cadical; CPU shared with cargo-mutants `-j 2`):
  - G1 distinct paths: depth 4 (40 min), depth 2 (30 min).
  - G6 schema evolution: 4 tags (15 min), 3 tags (30 min), 2 tags (30 min).
  - G6 canonical: 6 bytes as a symbolic `Vec` (about 27 min), 4 bytes as a `Vec` (15 min), 4 bytes as a fixed array (20 min).
  - G6 identity: 2 fields x 4 bytes (20 min; the first attempt also hit an unwinding bound).

  The likely cause is CBMC unrolling the varint writer and `Vec` growth at every call site. These claims are random-engine only until a run completes; the attempted bounds are recorded in the harness rustdoc.
- Kani on G4 cache: not run. Kani 0.68 ICEs on any reachable `catch_unwind` (intrinsic signature check). The `unit-testing` mock, which is reachable from the cache's `dyn` values, contains one. Minimal repro: a harness calling `std::panic::catch_unwind(|| 1)`.
- F2: no separate slot-state exhaustion harness. Part A, which drives every slot state through the model, and the G4 cache harness (get/exists/put/delete/touch from absent and present initial states, against a recording host) cover it.
- F3, legacy `store::*` drop-write positions: not injected. On chain a panic traps without unwinding, so a drop write never runs in a failed receipt. In the unwinding mock the drop runs during the unwind and the simulated revert restores the snapshot anyway.
- A1, A2, A4 and the gas notes: another agent's scope, not duplicated.
- The ft-v2 example workspace (`examples/fungible-token`) was not part of the mutation test command, so the migration paths only it uses show up as missed mutants.
- `quiet()` in c4/c5/f3 swaps the process-global panic hook. Under the parallel test runner, interleaved calls can leave the silent hook installed for the rest of the binary, which hides panic *messages* (not failures) from unrelated tests. Not changed.

### Test counts (end of round)

- Lib: 617 (572 at the start of the round).
- `state_v2_verify`: 13 (new).
- Doctests: 344.
- `state_v2_exchange` 34, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15, `state_v2_migrate` 5: all unchanged and passing.
- ft-v2 example: 16 migration tests plus 1 nested_trap, passing with the new prefix guard.

`abi_tests` and `deny_unknown_arguments_tests` still fail for the environment reasons noted above.


## Merge and re-verify (Oct 18)

`proto/storage-v2-d9` (7c701af7: API rounds 2 to 4 plus the audit branch) merged into `proto/storage-v2` (c822d587: Soundness round), merge base 1661f6b5. Then the reference model and every harness were ported to the merged API and re-run at the Soundness round bounds.

### The merge (56acbd9b)

- **One textual conflict, `PROTOTYPE_NOTES.md`.** Both sides had appended after "API round (Oct 17)". All sections are kept, in the order they were written: API round 2, API round 3, Audit round, API round 4 (d9 commits 18:31 to 19:47), then Soundness round (23:26).
- **`docs/storage-v2-spec.md`:** only d9 had changed it, so it was taken whole.
- **Auto-merged, checked by hand:**
  - `migrating.rs`: the M4 `check_v1_prefix` guard (in `bind`, `decode_value` and `__write_pending`) sits beside d9's read-only `new_side`/`old_side`, the `*_mut_unchecked` accessors, `target_try_insert`, and the two-sided `remove`. They do not interact.
  - `key.rs` and `state/mod.rs`.
  - The deletion of ft-v2 `tests/nested_trap.rs` (T7).
- No production code changed in the merge, or after it except rustdoc.

### Expectations changed, and why

Every harness keeps its named property. Only expected numbers and API spelling changed. Harness function names are kept (e.g. `c5_remove_some_*`), so the Soundness round table still maps one to one. Two names are the exception, because they asserted behaviour that is now false: `c2_insert_new_on_an_existing_record_panics` became `c2_try_insert_on_an_existing_record_is_refused`, and `g7_at_mut_reads_the_record_current_shape` became `g7_at_mut_is_one_existence_check_current_shape`.

**Reference model (`model.rs`), from the spec's rules:**

- **Record slot (`RSlot`).** A record of a map whose value owns storage lives at its record key, or, while `remove_step` is clearing it, at its flag key `record_key ++ 00`, never both. The flush follows the spec:
  - a clearing record removes its record key, then writes the flag key (key order, F4);
  - a record no longer clearing writes its record key, then removes the flag key;
  - a delete removes whichever key holds it, and both if it was never read;
  - F1: a delete while the location is unknown makes a later write also remove the other key.
  - Record bytes no longer carry a `00 00` flag; `CLEARING_FLAG` is gone.
- **Flag probe on a miss.** A miss at the record key is followed by a read of the flag key for `get`, `get_mut`, `entry`, `remove`, `remove_step`, the retyped `get_mut`, record pages and views, and by a `storage_has_key` of the flag key for `contains_key` and `at_mut`.
- **`at_mut`** is an existence check: no read, `None` on a miss after the 2nd check, a panic if clearing.
- **Map headers (T4).** A `Map` header (`flat`, `recs`, each `items`) carries a revision. It is a bare `u32` while the revision is 0 and 12 bytes after. Every removal bumps it: `remove`, `discard`, the bounded clear once per entry, and `clear` once. An emptied map writes its header (len 0) instead of removing it. The header goes, revision and all, only with the record that owns the map, including when `remove_step` finishes. `Vector` is unchanged: a bare header, removed at 0.
- **`revision` in pages.** It is part of the compared `Ret::Page`, `Ret::KeyPage` and `Ret::RecPage`.
- **`try_insert` (T1).** New ops `FlatTryInsert` and `LookTryInsert`, and `RecTryInsert` (replaces `RecInsertNew`). An existing key is `Occupied` after the index read (`Map`) or 1 `storage_has_key` (`LookupMap` leaf), with nothing written. A record being cleared is `Occupied` on `Map`, since its index is there. `would_panic` no longer lists it.
- **`remove_step` (T3).** `Ret::Bool(true)` means `Removal::Pending`.
- **Generation stamps (A2).** No model case is needed: every op takes a fresh handle from the root, so none is ever stale. The model docs point to C5's `Escaped` attempt.

**Cost formulas (`cost.rs`, `g7_stated_cost::formula`, used by G3 and G7):**

| Row | Before | Now | Why |
|---|---|---|---|
| `RecGet`, absent or clearing | 1 read | 2 reads | flag-key read on a miss |
| `RecContains`, absent or clearing | 1 has_key | 2 has_keys | flag-key check on a miss |
| `RecEntryDefault`, new | (2, 4, 0, 0) | (3, 4, 0, 0) | record key, flag key, header |
| `RecGetMut`, absent | (1, 0, 0, 0) | (2, 0, 0, 0) | flag key |
| `RecTryInsert`, existing | panic (not a row) | (1, 0, 0, 0) | T1: `Occupied` after the index read |
| `RecAtMutInsert`, absent | (1, 0, 0, 0) | (0, 0, 0, 2) | existence checks, no read |
| `RecAtMutInsert`, present, item exists / new | (2, 1, 0, 0) / (3, 4, 0, 0) | (1, 1, 0, 1) / (2, 4, 0, 1) | 1 has_key instead of the record read |
| `RecRemove` | reads 2 + m; header removed iff m > 0 | reads lookups + 1 + m; header removed iff it exists (m > 0 or revision > 0) | flag probe; T4 header |
| `RecRemove` / `RecRemoveSome`, absent | 1 read | 2 reads | flag key |
| `RecRemoveSome`, unfinished | (2 + n + 1, (n > 0) + first, 3n, 0) | (lookups + 1 + n + 1, (n > 0) + first, 3n + first, 0) | the first unfinished call removes the record key and writes the flag key; lookups = 2 once flagged |
| `map_unlink(Only)` | (2, 0, 4, 0) | (2, 1, 3, 0) | T4: the header is written, not removed |
| G3 bytes, flat map | header 4 | header 0, 4 or 12; a removal writes 12 + the replaced header; Last = Only | T4 |

Here "lookups" is 1 for a live record and 2 for a record being cleared. "first" is 1 on the first unfinished call, else 0.

**Other harnesses:**

- **`g7_at_mut_is_one_existence_check_current_shape`:** (3, 4, 0, 0) with a read of the record became (2, 4, 0, 1). It now asserts no read of the record key, a `storage_has_key` on it, and, for an absent record, exactly [has_key(record), has_key(flag)].
- **C2 (`assert_no_orphans`):** accepts a live record's flag key and emptied headers, and asserts that each live record is stored at exactly one of its two keys.
- **C2 refusal test (renamed):** `Err(Occupied)` instead of a panic, for `Map` and `LookupMap` of records, in the same call and a later call. It newly asserts the record keeps its child and storage is unchanged.
- **C5:**
  - flag writes are counted at `record ++ 00`;
  - the first unfinished call removes the record key exactly once, before the flag write;
  - later unfinished calls touch neither key, and no clearing call writes the record key;
  - the finishing call removes the flag key, or the record key if it never went pending;
  - new attempt `Escaped(j)`: a handle to the record's map made before `remove_step` in the clearing call must panic through the generation check after `Pending`;
  - `c5_pinned_budget_zero_makes_no_progress`: the first call's writes + removes went from 1 to 2.
- **G6 canonical:** the `is_clearing` prefix skip is gone, because records carry no flag. The claim is stricter now.
- **M4 hazard (`hazard_an_unwrapped_empty_prefix_v1_map_overwrites_the_v2_root`):** the unwrapped v1 map is now a `#[key(1, legacy)]` root field (T7: a bare v1 type at the root is a derive error). The hazard reproduces: `legacy` stores Borsh bytes and nothing checks the prefix.
- **A3, A5, C1, C3, G2, G4, C6, C4, F3:** API renames only, or no change. A5's printed bytes per attempt went from `[(7, 262166), (7, 6), (12, 24)]` to `[(8, 262166), (7, 6), (11, 26)]`:
  - attempt 1: +1 remove, the record key when the record moves;
  - attempt 3: −1 remove, the emptied outer map's header now stays (−4 B); +8 B, the 12-byte items header; −2 B, the record no longer carries `00 00`.

  The hazard's assertions hold unchanged. The generation stamp touches neither A5 test.

### Findings

- **No regression.** Every named property holds on the merged code, and nothing was pinned as a regression.
- **New, a cost note (not a bug).** F1's rule ("a delete with the original state unknown") also fires without any `discard`. `try_insert` writes a record blind (`Map` decides on the index), so `try_insert` then `remove`/`remove_step` then `try_insert` of the same key in one call makes the second insert also issue a `storage_remove` of a flag key that never existed (value 0, replaced none). It is correct, 1 extra host call. The spec's F1 sentence names only a blind `discard`. The differential harness found it on its first failing trace, and the model now encodes the rule as the spec states it.
- **Sensitivity.** Two hand mutations of `cache.rs`, reverted:
  - `check_live` disabled: C5 fails on `Escaped(0)`.
  - F4's order reverted (flag write before the record-key remove): Part A (fixed and random), C5 and G4 (2 harnesses) fail.

### Harness results (merged code, random engine, Soundness round bounds)

| Harness | Bound | Result |
|---|---|---|
| Part A `differential_against_reference_model` (+ fixed sequences) | 2,000 histories of up to 60 ops | pass |
| G1 `g1_field_tags_are_prefix_free_over_u32` | 20,000 | pass |
| G1 `g1_addressable_distinct_paths_distinct_keys` | 20,000 | pass |
| G1 `g1_production_keys_equal_spec_keys` / `g1_long_paths...` / `g1_nested_get...` | 20,000 / 5,000 / 2,000; fixed cases | pass |
| C7 `c7_admitted_v1_prefixes_never_alias_v2_keys` | 100,000 | pass |
| C7 `c7_production_v1_keys_vs_production_v2_keys` + `m4::*` derive tests | 20,000; fixed | pass |
| G2 `g2_local_op_touches_only_its_own_path_and_bookkeeping` | 3,000 | pass |
| G3 `g3_no_debt_counts_depend_only_on_the_ops_own_key` | 5,000 | pass |
| G3 `g3_no_debt_after_10000_random_operations` | 3 histories x 10,000 ops | pass |
| G4 `one_write_per_key...` / `nothing_happens_in_drop` / `cache_flush...` | 2,000 / 2,000 / 20,000 | pass |
| G5 `g5_page_costs_two_reads_plus_two_per_entry`, `g5_page_of_64_on_an_aged_map` | 5,000; fixed | pass |
| G6 identity / canonical / large / schema evolution | 20,000 / 50,000 / 20,000 / 20,000 | pass |
| G6/D2 derive (`state_v2_verify`) | 2,000 each; required 500 | pass |
| G7 `g7_stated_cost_beside_a_100kib_neighbour` (+ 3 pinned) | 20,000 | pass |
| C1 `c1_spliced_keys...` + baseline | 5,000; fixed | pass |
| C2 `c2_no_orphans...` / `c2_removed_record...` / nested (`state_v2_verify`) / refusal | 2,000 / 2,000 / 1,000 / fixed | pass |
| C3 `c3_a_mid_call_flush_anywhere_loses_no_write` (+ fixed) | 2,000 | pass |
| C4 `c4_storage_delta...` (+ 2) | 500 histories | pass |
| C5 `c5_remove_some_isolates...` (+ pinned n = 0) | 3,000 | pass |
| C6 `c6_paging_under_churn...` (+ 2 pinned) | 5,000 | pass |
| F3 `f3_a_failed_call_leaves_no_trace` | 2,000 | pass |
| A3, A5 pins (`state_v2_verify`) | fixed | hazards reproduce |

### Kani (0.68, host, `NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo kani --tests --lib -p near-sdk --features unstable --harness <name>`)

| Proof | Result | Verification time | Wall (with build) | Soundness round |
|---|---|---|---|---|
| `g1_field_tags_are_prefix_free_over_u32` (every pair of u32 tags) | VERIFIED | 4.8 s | 14 s | 6 s |
| `c7_admitted_v1_prefixes_never_alias_v2_keys` (bounds as in the Soundness table) | VERIFIED | 131.9 s | 137 s | 152 s, CPU shared then |

The harnesses that did not finish under Kani in the Soundness round were not retried.

### Doc-corrections triage (the Soundness round's 12, annotated in place above)

| # | Status | Action |
|---|---|---|
| 1 last-key remove skips the slot read | still true | `Map::remove` rustdoc fixed; spec open |
| 2 blind remove in `LookupMap::discard` / `LookupSet::remove` | still true | rustdoc fixed (and for a record value: 2 removes, the flag key); spec open. Round 5: a record value is now read first (1 read, 1 remove, 0 if absent); spec states it |
| 3 blind remove in `Lazy::remove` | still true | rustdoc fixed; spec open. Round 5: still true for plain values; a record value is read first, then 0 removes if empty |
| 4 unfinished clear reads one more slot | still true | `remove_step` rustdoc fixed; spec's per-call line open. Round 5: spec states it (bookkeeping bound and per-call table) |
| 5 flag written only on the first unfinished call | resolved by round 2 | none |
| 6 `at_mut` rustdoc stale | resolved by round 2 | none |
| 7 `swap_remove` elides an identical rewrite | still true | rustdoc fixed; spec open |
| 8 limit 0 returns `next = None` | still true | `Map::page` rustdoc fixed; spec open |
| 9 measured sha256 per op | still true, counts unchanged | spec only (no state rustdoc states a count) |
| 10 cursor paging can return keys twice | still true | `Map::page` rustdoc fixed; spec's Pagination names only the skip |
| 11 Lazy values outside the budget | partly resolved (spec says Lazy does not count) | rustdoc fixed; spec open: "`n = 0` clears nothing and sets the flag" is false for a record holding only Lazy values or empty maps. Resolved by round 5: the budget counts host removes, `Lazy` included |
| 12 `n = 0` never progresses | resolved by round 2 in the spec | rustdoc now says it too. Superseded by round 5: `n = 0` does not compile, 1 and 2 panic |

Left for the orchestrator, design doc only: 1, 2, 3, 4, 7, 8, 9, 10, 11, plus the F1 cost note above. `verify/cost.rs` `DOC_DIFFS` keeps its indices and marks 4 and 5 (the list's items 5 and 6) as resolved.

### Test counts (end of merge round)

- Lib: 636 passed, 1 ignored (`promise::tests::test_yielded_promise_can_chain_then`, pre-existing).
- Doctests: 347 passed, 7 ignored, plus 5 standalone `compile_fail`.
- `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15, `state_v2_migrate` 9, `state_v2_verify` 13.
- ft-v2 example: 16 migration, plus 2 doctests (1 + 1 standalone `compile_fail`).
- near-sdk-macros: 49, plus 1 doctest (4 ignored).
- `proto-wasm-check`: wasm32 check passes, no warnings.
- `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reasons as before.

### Deliberately not done

- **No cargo-mutants re-run.** The Soundness round's mutation tables describe the pre-merge code. Only the two hand mutations above were tried.
- **libFuzzer / cargo-bolero:** still unavailable (no toolbox, Docker not running). Random engine only.
- **Kani:** not retried on the harnesses that did not finish before (G1 distinct paths, the G6 trio), nor on G4 cache (the `catch_unwind` ICE).
- **Miri:** not re-run on the audit tests after the merge. The merge touched no `unsafe` code.
- **Untouched:** the near-contract-sim gas runs; `docs/storage-v2-spec.md` (the design-doc corrections are listed above for the orchestrator); the Claude Doc.
- **No `rustfmt`:** the tree was not rustfmt-clean on either side before the merge, and reformatting would bury the diff.
- **`quiet()`:** still swaps the global panic hook (unchanged, as noted in the Soundness round).

## API round 5 (Oct 18, hazard fixes)

Two settled design fixes for the Soundness round's hazards A3 and A5 (and C5 with it). Commits: `a3655a71` (A3), `b4c69bdb` (A5), then this notes/spec commit.

### A3: removal fails closed on unknown collection tags

**Rule.** Removing a record whose stored bytes carry a tag the record does not declare, and does not list in `reserved(n)`, holding a collection kind marker panics before anything is removed. The message names the tag (a tag path such as `2 > 2` when nested) and the collection type, and gives the two fixes: roll forward to a version that declares it, or list it in `#[near(storage, reserved(N))]`. Unknown scalar tags keep the old behaviour (dropped with the record). A `reserved(n)` tag is known and empty: listing a collection tag there is the developer's statement that the version owning that collection cleared it, so the SDK has nothing to check and nothing to clear.

**Mechanism.**
- `record::marker`: `0x81` Map, `0x82` LookupMap, `0x83` Set, `0x84` LookupSet, `0x85` Vector, `0x86` Lazy, `0x87` GroupedMap. All are at least `0x80`, so no `bool` or fieldless enum under 129 variants encodes as one.
- `RecordWriter::new()` writes `varint(tag) 01 kind` for every non-inline field (`root::encode_field` calls `push_marker(tag, T::KIND)`); `RecordWriter::root()` writes none, since a root is never removed. Decoding skips markers; they are part of no key.
- The derive generates `Value::__unknown_collection(body)`: per declared tag it recurses into that field's type (so a nested record's own unknown tags are found), reserved tags return `None`, and any other tag goes to `root::unknown_field_collection` (a 1-byte marker, or a value that parses as a canonical record holding a marker, searched recursively).
- `collections::guard_removal::<V>` reads the stored bytes through `cache::stored_bytes` (no extra read if the call already read them) and panics on a hit. Called by `remove`, `discard`, `replace`, `remove_step`, `Map::clear` / `clear_budgeted` of record values, `Vector` `pop` / `swap_remove` / `clear` / `clear_children_some`, and `Lazy` `remove` / `clear_children` / `clear_children_some`.

**Bytes and ops, before and after.**
- `Account { nonce: u64, balances: Map }` record: 10 bytes to 13 (`01 08 <8> 02 01 81`). +3 staked bytes per collection or `Lazy` field per record: 30 NEAR per million records per field.
- `discard` of a record (`LookupMap` / `Map`): 0 reads, 2 removes (record key and flag key, blind) to 1 read, 1 remove (5 removes instead of 6 in `flag_key_is_dropped_by_replace_remove_and_discard`).
- Bulk clear of record values: per entry 1 read more, 1 remove fewer (3, was 4).
- `Lazy::remove` of a record value: 1 read more.
- `remove`, `replace`, `remove_step`: unchanged (they already read).

**Tests.** The A3 pin `a3_rollback_removal_of_a_record_with_an_unknown_collection_tag_panics` now asserts the panic (remove, discard, remove_step, clear, `LookupMap` remove/discard/replace) and that the rolled-forward schema removes the record and its children with no keys left. New:
- `a3_known_collections_and_unknown_scalars_still_remove`;
- `a3_reserved_collection_tag_is_known_and_empty`;
- `a3_nested_record_collections_are_checked_at_any_depth` (derive-level, two schemas each);
- `record::tests::unknown_values_classified`;
- `g6_kind_markers_written_preserved_and_checked_on_removal` (20,000).

The model writes `REC_ITEMS -> [0x81]` in `RecM::bytes()`, so the differential harness checks marker bytes exactly. Sensitivity: disabling marker emission fails the differential harness and 2 of the A3 tests.

### A5: the budget counts host removes

**Rule.** `remove_step(&k, n: NonZeroU32)` makes at most `n` `storage_remove` calls of any kind per call. Exact bound plus bookkeeping, for a depth-1 record with `F` Map/Set/Vector fields and `L` Lazy fields: at most `n` removes, `n / 3 + F + 5` reads, `L` `storage_has_key`, 3 writes per call. Nesting adds per entry 1 read of the nested record and its header reads.

**Mechanism.**
- Whole units, taken only if they fit in what is left: map entry 3; `Lazy` 1 (after a `storage_has_key`, so a resumed call does not re-pay); `Vector` slot 1 (the last also takes its header); emptied header 1, or 0 if never stored; finish 1 (`LookupMap`) or 3 (`Map`: record, index, last key slot); move to the flag key 1, reserved up front so a `Pending` call stays within `n`.
- `cache::delete_cost::<V>` and `cache::clearing_cost` compute the exact removes a pending delete adds, mirroring `flush_ops` (`Slot::remove_count`); a test-only `cache::pending_removes()` debug-asserts the two agree.
- `MIN_REMOVE_STEP = 3`: a map entry and a `Map` finish cannot be split, so `n` of 1 or 2 panics ("must be at least 3"), and `n = 0` is a type error (`compile_fail,E0308` doctest).
- `Map::clear()` is unchanged (unbounded).

**Op counts, before and after.** For the A5 shape (4 x 64 KiB `Lazy`, 3 map entries), per call (removes, value bytes):
- Before, at `n = 1`: 3 calls, (8, 262,166), (7, 6), (11, 26).
- After, at `n = 3`: 7 calls, (3, 131,095), (2, 131,080), (3, 6), (3, 6), (3, 6), (1, 12), (3, 23), 18 removes total. Each call is at most `n` removes and `n` x 64 KiB.

`a5_lazy_values_count_against_the_budget` (6 `Lazy`, `n` 3..=10, both maps) asserts 1 call if 6 + finish ≤ n, else ceil((6 + 1 + finish) / n) calls. `remove_step_budgets_across_calls`: `Map` at n = 7 removes [7, 6, 7] per call; `LookupMap` at n = 3 removes [1, 3, 2]. C5 now asserts exact per-call reads, writes and removes against a mirror of the rule, 1..=n removes per call, and the total of 3m + 1 + 3 + (1 if moved). The pinned `n = 0` test became `c5_budget_below_three_panics_and_three_progresses`. G7 and the model use the cost-based rule; C2 and diff generators draw `n` from 3 up.

### Deviations from the brief

- **The minimum is 3, not 1.** `NonZeroU32` alone would allow 1 and 2, which cannot remove a map entry (3 removes that cannot be split without breaking the clearing record's map iteration). `n` of 1 or 2 panics.
- **The A5 pin asserts ceil((K + 1 + finish) / n) calls, not ceil(K / n).** The move to the flag key (1) and the finish (1 or 3) are removes too, and the rule counts every remove. When K + finish fits in `n` it is 1 call (no move).
- **The bookkeeping bound is schema-dependent** (`F`, `L`, depth), not one constant.
- **C3 weakened.** A mid-call flush changes which removes are still pending, so a later `remove_step` in the same call can make different progress. The harness allows divergence only when a `RecRemoveSome` follows the inserted flush in that call, and counts it (0 to 2 of 2,000 per run; 0 in the final run).
- **The kind marker has a false positive.** A one-byte `u8`/`i8` with value `0x81`..`0x87` under an unknown tag fails closed, and so can an unknown scalar whose Borsh bytes parse as a canonical record holding a marker.

### Deliberately not done

- A blind `insert` of a record without collections, and `Lazy::set`, over bytes carrying an unknown collection tag: not checked (no read; adding one would cost every insert a read).
- Records written before round 5 carry no markers until rewritten (`get_mut`, `entry`); `at_mut` does not rewrite. No migration pass.
- The root carries no markers.
- The reference model has no schema with an unknown collection tag; the derive tests cover it.
- No prefix delete or generation component in paths (alternatives for A3); no budget for `Map::clear()`.
- Kani not run (another agent's job); no cargo-mutants re-run.

### Harness results (round 5 code, random engine, bounds as recorded)

| Harness | Bound | Result |
|---|---|---|
| Part A `differential_against_reference_model` (+ fixed sequences) | 2,000 | pass |
| G1 prefix-free / distinct paths / production = spec / long paths / nested get | 20,000 / 20,000 / 20,000 / 5,000 / 2,000 | pass |
| C7 admitted v1 prefixes / production keys | 100,000 / 20,000 | pass |
| G2 local | 3,000 | pass |
| G3 counts / 10,000 ops | 5,000 / 3 x 10,000 | pass |
| G4 one write / drop / cache flush | 2,000 / 2,000 / 20,000 | pass |
| G5 page cost (+ aged map) | 5,000 | pass |
| G6 identity / canonical / large / schema evolution | 20,000 / 50,000 / 20,000 / 20,000 | pass |
| G6 `g6_kind_markers_written_preserved_and_checked_on_removal` (new) | 20,000 | pass |
| G6/D2 derive (`state_v2_verify`) | 2,000 each; required 500 | pass |
| G7 stated cost beside a 100 KiB neighbour (+ 3 pinned) | 20,000 | pass |
| C1 spliced keys (+ baseline) | 5,000 | pass |
| C2 no orphans / removed record / nested (`state_v2_verify`, n 3..=5) / refusal | 2,000 / 2,000 / 1,000 / fixed | pass |
| C3 mid-call flush (+ fixed) | 2,000 | pass, 0 diverged |
| C4 `storage_delta` (+ 2) | 500 | pass |
| C5 `remove_step` isolates the record (+ below-three pin) | 3,000 | pass, exact per-call counts |
| C6 paging under churn (+ 2 pinned) | 5,000 | pass |
| F3 failed call leaves no trace | 2,000 | pass |
| A3 pin + 3 tests, A5 pin + shape test (`state_v2_verify`) | fixed | fixed: A3 panics, A5 within budget |

47 tests in `near-sdk/src/state/verify/` (was 46) and 16 in `state_v2_verify.rs` (was 13).

### Test counts (end of round 5)

- Lib: 639 passed, 1 ignored (same pre-existing ignore).
- Doctests: 348 passed, 7 ignored, plus 6 standalone (the new `remove_step` `compile_fail` for `n = 0`).
- `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15, `state_v2_migrate` 9, `state_v2_verify` 16.
- ft-v2 example: 16, plus 2 doctests.
- near-sdk-macros: 49, plus 1 doctest.
- `proto-wasm-check`: wasm32 check passes.
- `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reasons as before.

## Verification round 2 (Oct 18)

Better guarantees pass after API round 5: the impeccable-rust Testing, Chaos and Exhaustive verification sections. Started at 5030854b. One production change to the record format (the kind-marker fix), one compile-time API tightening (`Lazy` of a collection), a Miri fix to the unit-test mock (outside `state/`), and test work.

### Kind markers are wire-typed (eecbd7ec)

A field key is now `varint(tag << 2 | wire)`:
- Wire 0 is a scalar.
- Wire 1 is a kind marker; its body must be exactly 1 byte.
- Wire 2 is a nested record.
- Wire 3 is reserved and rejected (`RecordError::ReservedWire`).

Tags stay strictly ascending across all wire types, key < 2^34, and tag 0 is still rejected. The A3 removal check reads only wire-1 fields and recurses only into wire-2 fields, so no scalar can be mistaken for a marker. Before this, a 1-byte scalar 0x81..0x87, or a nested value that parsed as a record, read as an unknown collection and blocked removal. The kind bytes are unchanged (0x81 to 0x87). An unknown kind byte under wire 1 still counts as a marker (fail closed).

Changes:
- **Writer:** `RecordWriter` keeps each field's wire type (`push`, `push_record`, `push_marker`, `push_wire`), and unknown fields merge back in order with their wire.
- **Derive:** the derive calls `declared_field_collection::<T>(wire, body)`, which recurses only for wire 2.
- **Cost:** one-byte field keys now cover tags up to 31 (was 127). The 13-byte account example is `04 08 <8> 09 01 81`.
- **Spec:** the record-format grammar, the worked example, the staked bytes, the A3 paragraph and the decision row are updated.
- **Model:** `enc_record` writes `(t << 2) | wire`.
- **G6 harnesses:** the identity and canonical harnesses draw a wire per field. The marker harness is renamed `g6_kind_markers_are_wire_typed_preserved_and_checked_on_removal` and requires that no scalar or scalar-like-record ever fires. Unwind bounds are unchanged.

The former false positive is pinned by `record::tests::scalars_with_marker_bytes_are_not_collections` and `state_v2_verify::a3_scalars_with_marker_bytes_never_block_removal`. Re-adding the old heuristic fails 3 tests.

### Reference model owns Lazy, Set, LookupSet and Vector in records (da03611c)

`MRoot` gains two hand-expanded record maps:
- `xrecs: Map<u8, RecX>` (tag 6), where `RecX` holds `n`, `Lazy<u64>`, `Set<u8>` and `Vector<u64>`, and is Removable.
- `lrecs: LookupMap<u8, RecL>` (tag 7), where `RecL` holds `n` and `LookupSet<u8>`, and is not Removable.

The model mirrors them with `RecXM`/`RecLM`, `concretize_set`/`concretize_vec` and shared Vector helpers, and the budgeted clear is Lazy, then Set, then Vector, counted in host removes. There are 29 new ops, including remove_step on X records with the D9 flag key. `would_panic` covers writes to a clearing X record and out-of-bounds swap_remove.

New differential tests:
- `differential_fixed_nested_sequences`.
- `differential_generator_reaches_nested_ops`: at least 25 nested op kinds in 200 histories.
- `differential_nested_against_reference_model`: 2,000 nested-only histories.

Three hand mutations are each caught: skipping Lazy's has_key in the budgeted clear, a free Vector header, and an early `true` from the Vector clear.

### Miri

The runs used nightly Miri on the host, with `-Zmiri-ignore-leaks`, plus `-Zmiri-disable-isolation` for bolero, which calls `stat`. Stacked Borrows (SB) is the default model; Tree Borrows (TB) adds `-Zmiri-tree-borrows`.

| Target | Commit | SB | TB |
|---|---|---|---|
| 10 cache/handle audit tests (A1 ×3, A2 ×7) | eecbd7ec | pass | pass |
| Round-5 tests: remove_step ×2, flag key, unknown tags survive, C5 below-three | eecbd7ec | pass | pass |
| `record::tests` (9) | eecbd7ec, 2999a34d | pass | pass |
| G6 kind-marker harness | eecbd7ec | pass | pass |
| `state::tests::` (74) | eecbd7ec | **UB** (mock) | **UB** (mock) |
| `state::tests::` (74) | 2999a34d | pass (32 min) | pass (47 min) |
| `state::tests_gaps::` (27) | 780458ed | pass | pass |

Both findings were in the unit-test mock (master #1629), not in `state/`, and both were fixed in 2999a34d:
- **SB:** `MockedBlockchain::new` moved the `Box`-owned `HostFixture` after `HostCtx` had borrowed through it. Moving a `Box` retags it Unique and invalidates those borrows. It fired when a panic inside `catch_unwind` lazily created a fresh mock. The fixture now owns both allocations through raw pointers (`Box::leak`/`NonNull`) and frees them in `Drop`.
- **TB:** a test that catches a state panic unwound out of `near_sys::panic_utf8`, which is declared `extern "C"`, while the mock defines it `extern "C-unwind"`. near-sys now declares its imports `extern "C-unwind"`, which is identical on wasm32 (contracts use `panic = "abort"`). env.rs's `method_into_register` fn-pointer type follows. This is a cross-crate change to a published crate's fn item types, kept in its own commit so it can be dropped or upstreamed separately.

One Miri warning remains: an integer-to-pointer cast at mocked_blockchain.rs:284. It comes from the mock's u64-pointer host ABI, which mirrors wasm, and it is not UB under permissive provenance.

### Mutation testing, round 2

The settings are the Soundness round's: cargo-mutants 27.1.0, `--in-diff` of `git diff master...eecbd7ec -- near-sdk/src/state`, and the 7 production files of `state/`. Pass 1 ran the `verify::` lib harnesses with `BOLERO_RANDOM_ITERATIONS=300`, skipping `g3_no_debt_after_10000`, at -j 4. Pass 2 ran `--iterate` over the lib plus `state_v2_verify`, `state_v2_migrate` and `state_v2_exchange` at full iterations (auto timeout 288 s).

The three columns are:
- **Before:** the code and tests at eecbd7ec.
- **After:** after subagents A and B, the model extension and the litmus tests, measured by `--iterate`.
- **After-final:** adds the hand-proven kills of the two gap rounds (each mutation applied by hand, the new test watched fail, then reverted). The production files are unchanged by those rounds except 4a617de7, so no new sweep was run for them.

| File | n | Before caught / missed / timeout / unviable | After (measured) caught / missed | After-final caught / missed (all equivalent) |
|---|---|---|---|---|
| cache.rs | 226 | 134 / 27 / 19 / 46 | 142 / 20 | 160 / 2 |
| collections.rs | 393 | 186 / 84 / 14 / 109 | 231 / 40 | 269 / 2 |
| key.rs | 34 | 15 / 7 / 0 / 12 | 21 / 1 | 21 / 1 |
| migrating.rs | 92 | 49 / 23 / 5 / 15 | 65 / 7 | 67 / 5 |
| record.rs | 99 | 84 / 10 / 2 / 3 | 84 / 10 | 92 / 2 |
| root.rs | 60 | 35 / 16 / 0 / 9 | 50 / 1 | 50 / 1 |
| value.rs | 15 | 3 / 8 / 1 / 3 | 3 / 8 | 9 / 2 |
| **Total** | **919** | **506 / 175 / 41 / 197** | **596 / 87** | **668 / 15** |

Timeouts are 39 after and 41 before. They are counted as detected: mutants that loop or blow the 288 s budget, such as `get -> None` and `Slot::is_clearing -> false`.

Method for "before": tests were only added between the two runs, so a mutant missed after was also missed before. Only the 240 mutants on lines where "after" caught something were re-run at eecbd7ec (a crafted `--in-diff`, /tmp/storage-v2-review/vr2/before2.diff).

The gaps were closed in five groups:
- **Migrating (subagent A, 4 commits):** contains_key and bind, v1 LookupMap into a v2 LookupMap, the UnorderedMap/TreeMap/IterableMap legacy impls, and raw_root/root_orig/parse_orig.
- **Surface (subagent B, 2 commits):** the view-write check driven negatively, key MAX_LEN, the derive helpers (`varint_len`, `field_subpath`), Path and EnvHost, IndexIter, len/is_empty, At<LookupMap>, MAX_SUBPATH, and Lazy insert-new.
- **cache.rs (3 commits):** delete_cost, remove_count_as and stored_bytes accounting; take/delete/bury/delete_impl and slot freeing (a drop-counting leaf); and the check_live epoch and strict-prefix bounds.
- **Rest (5 commits):** the At views of Map, Set and Vector; the Value impls of Lazy, Set and LookupSet; kind markers and the unknown-collection message; Removal::is_done, Occupied and guard_removal's flag read; and NewLookupMap::target_try_insert.
- **Production fix found on the way (4a617de7):** `Lazy<Set<u8>>` (and any `Lazy<collection>`) compiled, then panicked at the flush with "entered unreachable code". `Lazy` now has the same const `CHECK` as Map, LookupMap and Vector, pinned by a `compile_fail` doctest.

The 15 survivors are all equivalent or unreachable. Each is listed with its reason in `.cargo/mutants.toml` `exclude_re` (12 patterns), which takes the list from 920 to 905 mutants:
1. `stored_bytes` guard `!s.merge -> true`: only a whole `put` sets Dirty, and it clears `merge` in the same statement. The test `gaps_cache_discard_after_in_place_mutation_still_checks_the_stored_bytes` pins the in-place path.
2. `root_orig -> None`: `raw_root` falls back to `read_root`, which serves the same cached bytes.
3. `record_at_mut` `MAX_SUBPATH > 0` → `>= 0`: only storage-owning values can be clearing.
4. `assert_not_clearing`, the same mutation: same reason.
5. `Path::is_empty -> false`: no empty Path is constructible.
6. `field_key` `|` → `^`: disjoint bits.
7. `write_varint` `|` → `^`: disjoint bits.
8. `FieldDefault::field_default -> Default::default()`: the body is already `T::default()`.
9. `Removable::clear_children_some -> true` (default body): every override of `clear_children` also overrides `clear_children_some`.
10. `__private::max` `>` → `>=`: equal arguments return the same value.
11. to 14. The default `LegacyMap::legacy_prefixes` body (4 mutants): all 5 impls override it.
15. `Migrating::detach -> ()`: unreachable, because Migrating is not Removable.

Observation, not changed: `Map::try_insert` over a record being cleared returns `Err(Occupied)`, while `LookupMap::try_insert` panics with the clearing message. Neither writes anything, so it is safe, but the two APIs disagree.

### Error litmus

Each main Err/panic path was neutered in a scratch worktree, and the `state::` lib tests (skipping the 10,000-op G3) plus all `state_v2_*` suites were run:

| Path neutered | Result |
|---|---|
| CursorGone (cursor falls back to the start) | caught by 19, incl. `page_with_removed_cursor_returns_cursor_gone` |
| Occupied (try_insert overwrites) | caught by 14 |
| try_insert's clearing check (LookupMap) | caught by 1 (`flag_key_is_dropped_by_replace_remove_and_discard`, direct) |
| `required` Missing → other error | caught by 14 |
| `required` Missing names the wrong tag | caught by 14 |
| `legacy` field Missing → other error | **survived** → `gaps_legacy_field_missing_is_missing_naming_the_tag` |
| `required` on a collection field check deleted | **survived** → `gaps_required_on_a_collection_field_is_refused` |
| Kind-marker refusal (A3 panic) | caught by 2 (`state_v2_verify` A3) |
| Clearing guard in `assert_not_clearing` | caught by 7 |
| Clearing guard in `record_at_mut` | caught by 2 |
| Generation-stamp panic (`check_live`) | caught by 3 |
| v1-prefix check (`check_v1_prefix -> Ok`) | caught by 9 |
| v1-prefix bind guard | caught by 6 |
| `storage_usage` guard | caught by 6 |

Each of the two new tests was confirmed to fail under its neuter (780458ed). The scripts are in /tmp/storage-v2-review/vr2/litmus/.

### Kani G6 (launched, not awaited)

The three G6 harnesses run uncapped, one after another in a single detached shell loop. The unwind bounds are unchanged:
- `g6_record_encode_then_parse_is_identity` (unwind 18).
- `g6_record_parse_accepts_only_canonical_encodings` (unwind 7).
- `g6_schema_evolution_defaults_required_unknown_reserved` (unwind 10).

Launch details:
- **Worktree:** `.claude/worktrees/storage-v2-kani-g6`, detached at the commit that adds this section.
- **Command per harness:** `NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo kani --tests --lib -p near-sdk --features unstable --harness state::verify::g6_layout::<name>`.
- **Detaching:** the `trap '' HUP` + `perl setsid` + `caffeinate -i` recipe, with no timeout.
- **Logs:** `/tmp/storage-v2-review/kani/<harness>.log`.
- **PID:** the loop's PID is in `/tmp/storage-v2-review/kani/g6.pid`, and the start time is in `g6.start`.

The G1 run in `storage-v2-kani` was left alone.

### Deliberately not done

- **libFuzzer / cargo-fuzz:** Docker is down (the VM disk is full), so no fuzz targets were run. bolero ran with its random engine only.
- **No new full mutants sweep after the gap rounds:** the after-final column combines the measured after numbers with hand-proven kills. The production files changed only by 4a617de7 (`Lazy` CHECK, +1 mutant).
- **No Miri on the integration suites** (`state_v2_*`) or the whole lib. The matrix covers the state lib modules where the unsafe cache and handle code lives.
- **The near-sys `C-unwind` change and the mock fix are not upstreamed:** they are master-mock issues, kept in one commit.
- **Map/LookupMap try_insert inconsistency left as is** (see above).

### Test counts (end of verification round 2)

- Lib: 705 passed, 1 ignored (was 639 at round 5: nested-model differential tests, tests_gaps 27, tests_gaps_cache 14, tests_gaps_rest 19, record and model tests).
- Doctests: 348 passed, 7 ignored, plus 7 standalone. The new one is the `Lazy<collection>` `compile_fail`.
- `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15, `state_v2_migrate` 17 (was 9), `state_v2_verify` 17 (was 16).
- ft-v2 example: 16, plus 2 doctests.
- near-sdk-macros: 49, plus 1 doctest.
- `proto-wasm-check`: wasm32 check passes.
- `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reasons as before.

## API round 6 (Oct 18, flush perf)

A performance-only round, with no API or format change. The Oct 18 gas re-measurement bisected a
wasm-instruction regression to the D9 flag-key commit (`d4f2846c`): +10.4 Ggas on `Map` insert,
+3.9 on update, +16.2 on remove and +2 to +4 on `LookupMap` ops, all leaf rows with no extra host
call. Branch `proto/storage-v2-perf` from `5030854b` plus the gas harness. Code commit
`19050adb`. Evidence and tables are in GAS_MEASUREMENTS.md, "Flush regression fix (Oct 18, 2026)".

### Cause (measured, one variant per suspect on the real runtime)

- `Cache::load` copied the key before every host read (`key.to_vec()`, even when not at the flag
  key), and every read looked up its slot twice (`entry_slot`, then `load`). This one is on the
  read path, which is why `balance_of` (no writes) also moved at D9.
- `Slot::flush_ops` returned a `Vec<FlushOp>` per slot, with a copied key in every op.
- These two together reproduce the D9 delta on all six write-path rows (within 0.1 Ggas on five,
  1.1 Ggas past it on `Map` remove). The earlier "re-encodes every dirty slot" reading does not
  hold for contracts: the flush encodes each Touched/Dirty slot once, as before D9. The second
  encode is in `has_pending_writes`, which only `state::testing` calls.
- Also removed: the flush cloned every slot key before its loop. That cost predates D9 (3.0-6.8
  Ggas per row).

### Change (`near-sdk/src/state/cache.rs` only)

- `Cache::load(key, flaggable)` replaces `entry_slot` + `load`. One slot lookup. The key goes to
  the host as is, and the flag key is built only for a slot known to be at it. Callers:
  `get_impl`, `stored_bytes`, `read_root` (`flaggable = false`).
- `Slot::flush_ops(key) -> Vec<FlushOp>` becomes `Slot::for_each_flush_op(f)`. `FlushOp<'_>` is
  `Write(At, &[u8])` / `Remove(At)` with `At::{Key, Flag}`, so emitting an op copies nothing. The
  flush resolves `At::Flag` to `flag_key(key)` only when such an op occurs (a clearing move, or a
  blind delete of a record). A verbatim clearing move (review F3) borrows `orig` through a `Cow`
  instead of copying it. `has_pending_writes` and the `pending_removes` debug assert count
  through the visitor.
- `flush` destructures the cache (`host`, `slots`, `root`) and walks `slots` in place. The root
  key is compared as a slice. Nothing in the loop inserts a slot, and a host call cannot reach
  the cache (its `RefCell` is borrowed for the whole flush, as before).

### What did not change

- Op order: still key order, the record key before its flag key (review F4), the root last.
  Every op-log assertion keeps its exact numbers. No storage op, byte or trie-node count moved
  in any measured row.
- `remove_count` / `remove_count_as` are untouched and still agree with the visitor.
- No new `unsafe`. The flush keeps its existing `sp.as_ref()` deref under invariant 1, and
  `has_pending_writes` keeps its existing one.

### Result (v2 h16, 10k keys; Tgas whole receipt, op-only in Ggas)

| row | Oct 17 | before | after | op-only after − Oct 17 |
|---|---:|---:|---:|---:|
| `Map` insert new | 1.794 | 1.813 | 1.796 | −2.9 |
| `Map` update existing | 1.419 | 1.431 | 1.422 | −2.7 |
| `Map` remove non-last | 2.145 | 2.175 | 2.150 | −0.4 |
| `LookupMap` insert new | 1.312 | 1.321 | 1.315 | −2.1 |
| `LookupMap` update | 1.320 | 1.330 | 1.323 | −2.1 |
| `LookupMap` remove | 1.374 | 1.385 | 1.377 | −2.0 |

`v2_h16.wasm` is 164,513 → 163,361 B (−1,152 B; Oct 17 158,806 B). The T4 revision and A2 stamp
instruction costs (+1 to +6 Ggas per row) are still there. Removing the pre-D9 key clone more
than covers them.

### Test counts (end of round 6)

- Lib: 639 passed, 1 ignored. Doctests: 348 passed, 7 ignored, plus 6 standalone.
- `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15,
  `state_v2_migrate` 9, `state_v2_verify` 16.
- ft-v2 example: 16, plus 2 doctests. near-sdk-macros: 49, plus 1 doctest.
- `proto-wasm-check`: wasm32 check passes. `ext` and h8 gas contracts build.
- `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reason as before
  (the example crates believe they are in the outer workspace).

### Deliberately not done

- No change to T4 (revision header) or A2 (generation stamps). They are outside this regression.
- `free_slots` still collects the slots into a `Vec` (one allocation per call, not per key).
- Not re-measured: h8, the `ext` rows and the 1k/100k sizes. They run the same code paths.
- Not re-run: the Kani / bolero harnesses. Long Kani runs are in other worktrees, and this round
  changes no state transition, only how the ops are handed to the host.

## Merge of the perf branch (Oct 18)

`proto/storage-v2` at `63d62e86` (verification round 2) merged with `proto/storage-v2-perf` at
`2096f6e3`, which holds the Oct 18 gas re-measurement (`6c622757`, `cb867b4f`), the flush fix
(`19050adb`) and API round 6. Merge commit `40124811`.

**Conflicts.**
- **`near-sdk/src/state/cache.rs`: none.** `proto/storage-v2` has not touched cache.rs since the merge base
  `5030854b`. The kind-marker and wire-type work is in record.rs, and the gap tests are in
  `tests_gaps_cache.rs`. So the merged cache.rs is byte-identical to the perf branch's, with its
  single-lookup, no-copy `load(key, flaggable)`, the `for_each_flush_op` visitor and the in-place
  flush. The verification-round tests use only the cache's crate API (`pending_removes`,
  `delete_cost`, `put`, `delete_entry`, ...), whose signatures the fix did not change, so they compile
  unchanged.
- **PROTOTYPE_NOTES.md:** both sides appended after API round 5. Kept in order: Verification
  round 2, then API round 6.
- **proto-gas, GAS_MEASUREMENTS.md:** only the perf side changed them, so they merged clean.
- **Follow-up edits in the merge commit:** the mutant-kill comments in `tests_gaps_cache.rs` cite cache.rs
  line numbers. The fix shifted them by 9 to 12 lines, so they were re-pointed using a line-diff
  map (all 13 references land on the same source line). They and the `c5_remove_some` doc also
  name `for_each_flush_op` now, instead of `flush_ops`. The unsafe-block line list in the Audit round stays as is,
  as a historical snapshot.

**Confirmation.**
- **Tests:** exactly the round-2 counts. Lib 705 passed, 1 ignored. Doctests 348 passed, 7 ignored, plus 7 standalone.
  `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_nft` 17, `state_v2_dao` 15,
  `state_v2_migrate` 17, `state_v2_verify` 17. ft-v2 16 plus 2 doctests. near-sdk-macros 49 plus 1 doctest. The
  `proto-wasm-check` wasm32 check passes. `abi_tests` and `deny_unknown_arguments_tests` fail for the same environment reasons as before.
  No op-log assertion was changed.
- **Harnesses** (random engine, default iterations, which are the bounds recorded in the round-5 table): all 50 lib
  `verify::` tests pass, including G3's 3 × 10,000 ops and both differential models, and so do the 17 `state_v2_verify`
  tests.
- **Gas** (`perf_rows`, 10k keys, on the merged code): every one of the six write-path rows is within 0.4 Ggas of the perf
  branch's "after" column, with ops unchanged. Wasm `v2_h16` is 163,398 B (+37 B). Table in
  GAS_MEASUREMENTS.md, "Flush regression fix", "After merge (63d62e86 + perf)".
- **Miri** (nightly, `-Zmiri-ignore-leaks`), on the 10 A1/A2 audit tests plus the 14 `tests_gaps_cache` tests,
  is 24/24 under Stacked Borrows (254 s) and 24/24 under Tree Borrows (279 s). No UB. One
  integer-to-pointer-cast warning comes from the unit-test mock's host ABI
  (`mocked_blockchain.rs:284`, `Mem::copy_in` on `panic_utf8`), not from `state/`.

**Not done here:** Kani (long runs still going in their own worktrees, and the fix changes no
state transition). No new mutants sweep. h8, `ext` and the 1k/100k gas sizes were not re-measured.

## libFuzzer round (Oct 18)

The bolero harnesses ran under libFuzzer for the first time, in the impeccable-rust toolbox (Docker was restored by pruning the Rancher VM's image store). Started at 30bbfe9f, branch `proto/storage-v2-fuzz`. Full table and triage: /tmp/storage-v2-review/fuzz-results.md.

### Command (worktree root)

    scripts/impeccable run env NEAR_SANDBOX_BIN_PATH=/usr/bin/true \
      CARGO_PROFILE_FUZZ_INHERITS=dev CARGO_PROFILE_FUZZ_OPT_LEVEL=3 CARGO_PROFILE_FUZZ_INCREMENTAL=false CARGO_PROFILE_FUZZ_CODEGEN_UNITS=1 \
      cargo bolero test --toolchain nightly -p near-sdk --features "unstable unit-testing" -e libfuzzer -T 600s -t 10s <target>

Setup notes:
- The workspace defines no `[profile.fuzz]`, so it is passed through Cargo env config.
- libFuzzer needs `--toolchain nightly`.
- Lib targets are named by full path (`state::verify::g1_addressable::...`). Integration targets use their path inside `state_v2_verify` (`g6_derive_added_fields_default_without_a_write`).
- `.with_iterations` is ignored under libFuzzer.
- `catch_unwind` harnesses work.

### Scope and result

- **Main campaign:** 36 targets, 216.8 M execs.
  - At 10 min: the 2 differential harnesses and all 22 G1-G7 harnesses (the brief's "seven" undercounted).
  - At 3 min: the 12 bug-class targets (C1-C7, F3, D2).
- **Re-fuzz:** 7 targets re-run on the fixed code, 17.7 M execs, all clean.
- **Properties:** no property violation in any harness.
- **Not run:**
  - `differential_generator_reaches_nested_ops`: its assertion runs after `for_each`, which never returns under libFuzzer.
  - The A3/A5 pins: fixed-input tests, not bolero harnesses.
  - The second `check!` in `g6_derive_reserved_tag_dropped_on_rewrite_only`: never reached under libFuzzer.

### Cache leak at thread teardown (bec58728, production change)

LeakSanitizer reported leaks at exit in 5 harnesses (g1_nested_get, g6_derive_reserved, c1, c2 removed-record, c5). In each case the artifact was the empty input, and the leak was one input's worth after millions of execs.

Symbolized, the leaks are:
- `Box<Slot>` from `Cache::slot` (cache.rs:316).
- The slot's decoded value.
- The mock read bytes.

Cause: `Cache` owned its slots, handles and graveyard as raw `Box::leak` pointers with no `Drop`. A call that ended without a flush lost them when the thread-local `CACHE` was destroyed.

Fix: `impl Drop for Cache` calls `free_slots(self.take_slots())`, the same path as `begin_call`.

Impact:
- Teardown-only. Wasm never runs thread-local destructors, and flush/`begin_call` are unchanged, so no gas path changes.
- `install_host`, which has only test callers, still empties the cache first.

Limitation: at thread exit, a value whose `Drop` touches another thread-local (a nested v1 write-on-drop through the env mock) could find it already destroyed.

Pinned by `tests_gaps_cache::gaps_cache_thread_exit_frees_unflushed_slots` (red without the Drop: 0 drops). The 5 targets re-fuzzed clean.

### Mocked gas exhausted across inputs (5d10f029, harness only)

C7 production failed after 30k inputs and D2 after 9.7k, both with `HostError(GasLimitExceeded)` from the MockedBlockchain:
- C7: `env::sha256` / `env::keccak256` in the hashed v1 keys.
- D2: the mocked panic host call behind the expected decode error.

Cause: neither harness reset the shared mocked context, so gas accumulated until the 300 Tgas prepaid limit tripped. The random engine's case counts stay under it.

Fix: refresh the context with `testing_env!` once `used_gas` passes 200 Tgas. A reset on every input made C7 take 130 s under `cargo test`. Both harnesses re-fuzzed clean, and the old crash inputs were replayed.

Other harnesses that make mocked host calls without a reset could hit the same wall in longer campaigns.

### Test counts (end of libFuzzer round)

- Lib: 706 passed, 1 ignored (+1, the pin).
- Unchanged: `state_v2_dao` 15, `state_v2_exchange` 36, `state_v2_ft` 16, `state_v2_migrate` 17, `state_v2_nft` 17, `state_v2_verify` 17.
- `proto-wasm-check`: passes.

### Deliberately not done

- No gas or code-size re-measure for bec58728: the change adds no executed code in a contract.
- The `__fuzz__/` corpora stay untracked in the scratch worktree, not committed.
- No push.

## Kani: shrunk bounds (Oct 20)

The four Kani harnesses that never finished at their full bounds were stopped by a reboot on Oct 19 with no verdict: G1 distinct paths after about 33 h, and the G6 loop after about 25 h on its first harness (`g6_record_encode_then_parse_is_identity`; the other two never started). This round adds shrunk-bound proofs beside them. The originals are unchanged and stay the random-engine tests.

Kani 0.68, CBMC 6.11, cadical, on the host (64 GB, 18 cores), run one at a time with `caffeinate -i` and a per-run cap:

    NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo kani --tests --lib -p near-sdk --features unstable \
      --harness state::verify::<module>::<name> [--exact]

Logs, with peak RSS per run: `kani-logs/` in the worktree (excluded from git).

### Results

"No verdict" below is a resource stop (a time cap, or CBMC out of memory). In no run did a property check fail, and no unwinding assertion fired in a finished run. Two runs went to the full 60-minute cap: canonical `len1` and evolution `written_unknown`. Every other no-verdict run was stopped by hand at 10-41 minutes, below the cap, to fit the session. `len2`, `len3`, the other nine evolution cases and the G1 family are untried at the full cap.

| Harness (new, `#[cfg(kani)]`) | Bound | Wall time | Verdict |
|---|---|---|---|
| `g6_record_encode_then_parse_is_identity_small_s00`..`_s04` (5 proofs, one per field shape) | the empty record and every one-field record: scalar or nested record with a 0- or 1-byte body, or a marker; symbolic tag 1..=127 and body byte; unwind 3 | 106, 140, 169, 105, 108 s (629 s total; 645 s wall with build; peak 34 GB) | **VERIFIED** |
| `g6_record_parse_accepts_only_canonical_encodings_small_len0` | the empty string | 0.6 s | **VERIFIED** (trivial) |
| `..._canonical_encodings_small_len1` | every 1-byte string | 60 min cap (14 GB) | no verdict (still in symbolic execution) |
| `..._canonical_encodings_small_len2` | every 2-byte string | 15 min cap | no verdict (still in symbolic execution) |
| `..._canonical_encodings_small_len3` | every 3-byte string | 4 attempts, 10-11 min each | no verdict |
| `g6_schema_evolution_defaults_required_unknown_reserved_small_{absent,written}_<decl>` (10 proofs) | one tag, absent or any `u8`, each of the 5 declarations | 60 min cap on the first case (`written_unknown`, 15 GB) | no verdict; 9 cases not run |
| `g1_addressable_distinct_paths_distinct_keys_small_s00`..`_s22` (23 proofs, one per shape of path `a`) | two paths of depth <= 1, tags 1..=127, every collection kind and table, `Fixed(0..=2)` / `Var` 0..=2 keys, any slot index, one constant placement hash | `_s00`: 15 min cap | no verdict (still in symbolic execution); the other 22 not run |

The depth-2 G1 variant the brief asked for was not attempted: depth 1 did not finish.

### Attempts that led there

| Harness | Attempt | Wall | Peak RSS | Outcome |
|---|---|---|---|---|
| G1 depth <= 1 | 1. symbolic depth, unwind 10 | 21 min | n/a | killed, still in symbolic execution |
| | 2. depths enumerated, unwind 4 | 12 min | n/a | CBMC out of memory (another session's 7-44 GB CBMC ran alongside) |
| | 3. each prefix encoded once | 11 min | n/a | CBMC out of memory (same contention) |
| | 4. concrete hop shapes, all 23 x 23 pairs in one proof | 41 min | 23 GB | killed, still in symbolic execution |
| | 5. split per shape: `_s00` probe | 15 min | 12 GB | cap, still in symbolic execution |
| | probe: one pair (`Map`, `Var` 2-byte keys) | 15 min | 21 GB | cap, in propositional reduction |
| G6 identity | 1. 2 fields, bodies 0..=4, unwind 6 | 11 min | 3 GB (another CBMC at 44 GB) | killed, symbolic execution in std's stable sort |
| | 2. same, unwind 4 | 10 min | 5 GB | killed, same |
| | 3. field count concrete | 8.5 min | 48 GB | CBMC out of memory |
| | 4. one field, 11 shapes (bodies 0..=4) in one proof | 10 min | 10 GB | killed, still in symbolic execution |
| | 5. split per shape, unwind 4 | 20 min | 41 GB | killed on the first shape |
| | probe: 1 scalar field, 1-byte body, unwind 3 | 3.3 min | 30 GB | VERIFIED: led to the shipped bound |
| | probe: 1 scalar field, 4-byte body, unwind 4 | 15 min | 48 GB | cap |
| | probe: 2 concrete 4-byte fields | 10.5 min | 50 GB | killed |
| G6 canonical | 1-3. 0..=3 bytes, three ways of fixing the field count | 10-11 min each | 5-6 GB | killed, still in symbolic execution |
| | 4. `len3`, body length concrete too | 10 min | 7 GB | killed |
| | 5. `len0`, `len1`, `len2` separately | 0.6 s / 15 min / 15 min | 0 / 9 / 8 GB | VERIFIED / cap / cap |
| | 6. `len1` at the full cap | 60 min | 14 GB | cap, still in symbolic execution |
| G6 evolution | 1. one tag, all 10 cases in one proof | 20 min | 9 GB | killed, still in symbolic execution |
| | 2. split per case: first case | 19 min | 10 GB | killed, still in symbolic execution |
| | 3. `written_unknown` at the full cap | 60 min | 15 GB | cap, still in symbolic execution |

### What the cost is

A proof over the record codec or the key builder costs far more than the size of its input suggests. One scalar field with a 1-byte body verifies in 18 s at 0.9 GB when only the tag is asserted, and in 3.3 min when the whole identity is asserted. The same field with a 4-byte body reaches 48-50 GB with no verdict. Every `Path` hop and every record field allocates a fresh `Vec`. CBMC models each allocation as its own object, and a symbolic tag or length makes the later offsets symbolic, so each read through a pointer becomes a case split over objects and offsets. Concrete shapes and one case per proof are what made the identity proofs finish.

### Not done, and what would help next

- **Depth-2 G1, 2-field records, bodies over 1 byte:** not attempted, since the smaller bounds did not finish.
- **CBMC options:** `--refine-arrays` and stubbing the std sort need `-Z unstable-options --cbmc-args ...`. The mbx `cargo` shim refuses `-Z unstable-options` (its `cargo metadata` probe fails), so they were not tried. The untested route is Kani's `[workspace.metadata.kani]` flag tables in `Cargo.toml`.
- **Cheaper harness shape:** a harness-side byte-array key builder proven equal to `Path` on concrete shapes would avoid the allocations.
- **Part 1 of the design doc** still says "two Kani proofs", which no longer matches. It was left alone: no status sentence there says proofs are running.

## Sim search (Oct 20)

**Question.** The mock-host harnesses already test storage v2's invariants. Does a stateful property search on near-contract-sim find anything they cannot? The sim runs nearcore's runtime in-process, on main @ b328d067, its first release after the breaking "primitives" API change. **Answer: no SDK finding.** Every campaign below came back clean once two harness bugs were fixed. That is a result about the cases tried, not a proof. What the sim added is coverage the mock cannot give, listed below.

**What was built** (`proto-sim-search/`, commands in its README):

- **Contract.** One v2 root, written with the real derive: a root `u64`, `Map<AccountId, Account>` (`Account` = `u64` + nested `Map<String, u64>` + `Vector<u64>` + `Lazy<Vec<u8>>`), a `LookupMap` and a `Set`. 29 methods cover the model's op set:
  - inserts, overwrites and removes at the root and below a record through `at_mut`;
  - `try_insert`, `entry().or_default()`, `get_mut`, `remove`, `discard`;
  - `remove_step(k, n)` with any `n`, and `acct_remove_chain`, which steps through self-calls;
  - cursor pages of all three iterables;
  - read views and a cache-bypassing `probe_has`.

  Each mutating method takes a `mode`: plain, panic before the flush, `state_write()` then panic, or `state_write()` then a root change (the model's MidFlush).
- **Harness.** Ops are an enum drawn by `plans(ops()).len(1..=12)`, with Settle gaps, default gas and default `Faults`. One `at_rest` check replays every receipt the contract executed (`Sim::receipts_in(..)`, execution order) through an in-test model:
  - Each return value must equal the model's.
  - Each panic must be one the model predicts: a write below a clearing record, `n < 3`, a nested key over `MAX_LEN`, or a planned panic.
  - A receipt that failed for any reason is dropped from the model.
  - The whole trie (`view_state`) must then equal the model's concretization, key for key and byte for byte. Keys come from the spec's grammar, ported from `model.rs` `spec_key`. This covers G2/C2 (orphans), F3/G4 (no trace) and C4 (bytes).
  - `storage_usage` must equal the empty contract's plus key + value + 40 per key (C4 accounting).
  - C5: a clearing record must have no record key, its bytes verbatim at the flag key, only keys it held when clearing began, and stay readable through `get`. Every `remove_step` receipt's host removes, from the real gas profile, must be at most `n`, and at least 1 on a present record (liveness: a Pending call that touched nothing would fail). A first call's Done or Pending must match the record's clear cost against `n`.
  - C6: every page must equal the model's key-vector slice with its revision, or be `CursorGone` exactly when the cursor key is absent. A walk at one revision must return no key twice.
  - Probe: `storage_has_key`, called through the contract, is false on the record, flag and child keys of every absent record.

  Two per-op checks run on isolated ops: a failed op left the trie and `storage_usage` unchanged, and `remove_step` never adds a key under its record. Coverage asserts that every method was reached. A per-op tally counts behaviour coverage.

**Runs (debug profile, Apple M5 Pro).**

| Campaign | Cases / runs | Wall time | Result |
| --- | --- | --- | --- |
| Smoke: hand-written witness (29 steps) + positive control (orphan planted with `set_state`) | 2 | 0.2 s | clean; the control is caught |
| Search, `gas::mixed`, seed 0x20261020 | 2,000 cases (368 grown by guidance) + 366 max-gas, 2,369 runs, 55,071 op calls (38,050 of them overlapped another op, so the per-op before/after checks count them ok without judging; the global replay judges them) | 440 to 447 s over two runs, 0.22 s per case (growth included); the rerun with the liveness check gave identical counts | clean, all 29 methods reached |
| Search, `gas::wide`, seed 0x20261021 | 2,000 cases, 2,005 runs | 85 to 95 s over two runs, 0.04 to 0.05 s per case | clean, all 29 methods reached |
| `scan_each` of 2 witnesses, `Faults::default()` | 477 variants, 48 injected; `acct_remove_chain` starved in 12 | 56 to 58 s | clean |
| Gas walk: each step of a 13-step witness from 50 Ggas up in 20 Ggas steps until it succeeds | 1,228 runs, 1,174 failing gas values, 714 of them dying after the flush's first write or remove | 60 to 67 s | clean |

Behaviour reached in the mixed search, counted per receipt:

- `remove_step`: 440 Pending, 1,422 Done; chained links: 48,862 Pending, 872 Done.
- Refusals: 406 writes below a clearing record, 172 calls with `n < 3`, 4 nested keys over `MAX_LEN`.
- `CursorGone`: 323 on `accounts`, 214 on `balances`, 123 on `members`; 69 pages continued from a cursor.
- Planned panics: 673 before the flush, and 634 after a hand flush. In 346 of those 634 the hand flush had issued writes or removes before the panic (the others had nothing dirty); all were reverted. There were 756 mid-call hand flushes.
- 975 `remove_step` receipts on a present record returned a value, plus the chained Pending links; each was judged against the budget and the liveness bound.
- Natural out of gas: 428, mostly bulk inserts and chain links.

The wide search gave similar counts. Logs are in `proto-sim-search/results/*_20261020.txt`. The scan, the gas walk and both searches were rerun after the liveness bound was added, and all stayed clean.

**Findings, triaged.** None is an SDK bug, so no production code changed.

1. **Harness bug, fixed:** `Model::exec` returned early for an absent record before applying the method's `mode`, so it predicted success for a planned panic, and missed the extra counter bump after a mid-call flush. Found by the first 48-case search (6 findings, all this cause). The fix splits `exec_body` from the mode wrapper.
2. **Harness bug, fixed:** cases grown by headroom guidance and max-gas growth put `0 blocks` between steps, even though the strategy draws only `Gap::Settle`. So ops overlap, and a chained `remove_step` interleaves with them. The replay walked ops in plan order, and the before/after per-op checks assumed isolation, which gave 8 false findings in a 128-case search. The fix replays from `Sim::receipts_in(..)` (execution order) and judges per-op before/after only when `OpDone::isolated()`. The global replay judges every overlapped op receipt by receipt (38,050 such ops in the mixed search).
3. **Spec vs brief, no finding:** the brief says a record mid `remove_step` "reads as absent". The spec says, and the chain agrees, that `get`, `at`, iteration, `len` and `try_insert` (which returns `Err(Occupied)`) still see it, while `get_mut`, `entry` and `at_mut` panic. The harness encodes the spec.

No `Known` was declared.

**What the sim covered that the mock could not.**

- **Real trie.** Exact state is compared against nearcore's trie through `view_state`, not against the mock's `BTreeMap`. Every key the SDK wrote is the one the spec grammar computes, on wasm built by `cargo near`.
- **Real storage accounting.** `storage_usage` matched base + Σ(key + value + 40) after every op of every case, including 64 KiB `Lazy` values, flag-key moves, 12-byte headers and grown records of about 500 entries.
- **Out of gas mid-op, at real positions.** The mock's F3 harness fails at the i-th write. Here nearcore ran out of gas at 1,174 positions, 714 of them inside the flush (writes or removes already issued), and the runtime reverted each. The rest of the plan then ran on the result, under every check.
- **The real `#[near]` wrapper on wasm.** This covers load and flush, `state_write()` mid-call, panics with `panic = "abort"`, the generated `clear_children_some` over `Lazy` + `Vector` + nested `Map`, and JSON round trips of `AccountId`, pages and `Option`s.
- **The `remove_step` budget, from the real gas profile.** Host removes per receipt were at most `n` in every call, chained calls included.

What it did not add: cross-receipt SDK state leaks are structurally impossible on nearcore, since each receipt gets a fresh wasm instance. So the sim confirms the runtime's revert, not the cache's in-memory hygiene, which the mock's F3 harness covers. Exact per-call read, write and remove counts (C5's mirror) stay with the mock.

**Sim fidelity limits (`skills/near-contract-sim/references/fidelity.md`, `faults.md`).**

- An injected fault never selects a transaction's first receipt. Most ops are one receipt, so `Faults::default()` lands only on `acct_remove_chain`'s self-calls: 2 to 4 injected cases per 2,000-case search, 48 variants in the scan.
- An injected `OutOfGas` runs the call with 1 gas, so it fails at contract load, before any host call ("A fault (`Tx::inject`) rewrites a receipt"). Mid-op exhaustion comes only from gas exploration and the gas walk.
- One shard, no congestion, so `Hold` reorders nothing among one contract's receipts.

**API adaptations the new sim needed** (proto-gas was written against the old API):

- `explore::{Harness, Op, Plan, plans, Faults, Gap}` replace the old `testing::Scenario` layer. `Op::submit` only submits; it must not `transact`.
- A custom world (`World { sim, base }`: `Clone + AsRef<Sim> + AsMut<Sim>`), so checks know the empty contract's storage usage. Check closures must name `OpDone<StoreOp, World>` explicitly.
- `Sim::settle` / `step` / `findings()` are the new verb names. Failures are classified through `ReceiptOutcome::failure() -> Failure`, with `fault` for forced ones.
- `Sim::receipts_in(..)` gives execution order across overlapping ops. `ReceiptOutcome::storage_ops()` gives real host-call counts.
- `Run::outcomes()[step]` tells whether a step succeeded, for the gas walk.
- The guidance and max-gas growth ignore the strategy's gaps, so a harness must not assume Settle-only isolation.
- `proptest::prop_oneof!` needs a weight on every arm or on none. `Just(..)` arms of a `Copy` enum are fine.
- The sim crate shares the simulator's `CARGO_TARGET_DIR`, and its `Cargo.lock` is copied from the simulator (committed with `git add -f`, since the root `.gitignore` drops lockfiles). The first build recompiled some of nearcore's dependencies; later builds take seconds.
## Sim upgrade search (Oct 20)

Branch `proto/storage-v2-sim-b`, crate `proto-sim-upgrade/` (README there has the commands).
near-contract-sim main @ b328d067 (the "primitives" explore API). Question: do the multi-call
features (redeploys with schema evolution, `Migrating` + `migrate_batch`, `remove_step` across
calls) have interaction bugs the single-call mock harnesses cannot reach?

### Setup

One contract in five wasms: v1 (`store::IterableMap<String, u64>` under a Borsh root), v2a
(`Migrating<IterableMap, Map>` + `groups: Map<String, Group>`, Group owning `members: Map`),
v2b (tag 3 removed and `reserved(3)`, `level` with `default = 7`, `label` with `default_fn`,
tag 4 re-used as `claimable` where v2a had `locked`, new `flags: Set` at tag 8), v2c (v2b plus
`#[key(9, required)]`), v3 (wrapper off, bare `Map` at the same tag). `drain(n, rounds)` and
`clear_loop(g, n, rounds)` chain detached self-calls forwarding their remaining gas.

17 ops (items, groups, `MigrateBatch`, `Drain`, `RemoveStep`, `ClearLoop`, `RemoveGroup`,
`Upgrade { to, migrate }` = deploy (+ `migrate` in the same tx from v1), `Migrate`), all signed
by the contract account. Every successful mutating receipt logs `ev v=<version> seq=<n>`
(`seq` is a root counter carried through every upgrade), so the checks fold every receipt of
the case, continuations included, in execution order through a model of the documented
behaviour. Records are modelled at the tag level (stored tags per version, unknown tags kept,
reserved dropped, defaults materialised on a rewrite, tag-4 provenance), `remove_step` by the
spec's host-remove budget rule, `migrate_batch` by `IterableMap`'s swap-remove order. Checks:
seq gapless; nothing the model forbids succeeded (write below a clearing record, A3 removal,
decode of a `required`-less record, wrong `migrate_batch` count or `remove_step` result);
every item read back through the deployed code; each item on exactly one side, by raw keys
computed from the spec (and the v1 key vector slot by slot; no v1 key once drained); group
records byte for byte at the record or flag key, member and flag entries, no keys under a
removed record; group views per version; claimable only what schema B wrote (intent); no
collection outlives its record (intent); `storage_usage` = account + code + Σ(key + value +
40); the items subtree equals the spec's staked-bytes formula; every failure documented.

### Runs

| run | cases | wall | per case | verdict |
|---|---|---|---|---|
| search from v1 (seed 0x101, len 4-14, upgrade weight 10) | 512 | 74 s | 145 ms | Known (retag, 3 cases) |
| search from v2a (0x102, weight 3) | 512 | 91 s | 177 ms | Known (retag, 115 cases) |
| search from v2b (0x103, weight 4) | 512 | 102 s | 200 ms | Known (retag 512; continuation refusal 3) |
| OutOfGas scan, 8 witnesses, `Faults::default()` | 480 | 45 s | 93 ms | Known (retag 342, A3 hole 50, continuation refusal 42) |

1,536 search cases + 480 scan variants, about 5.2 min of wall time. Searches landed a fault in
22 + 17 cases (OutOfGas lands only on continuation receipts; ~120 cases per search drew a fault
nothing fits). Coverage probe, counted per op run with shrinking (upper bounds): continuations
ran 731 times in the searches, 15 on other code than their first link (`drain` v2a->v2b,
v2b->v2c); the scan's `.blocks(0)` witnesses ran 1,027 cross-version links (clear_loop v2a->v2b
480, v2b->v2a 147; drain v2a->v2b 400). `remove_step` resumed a clearing record 284 times in
the searches (32 under another schema than the one that started it) and 1,266 in the scan (495
under another schema). Writes below a clearing record were refused 150 times, A3 refusals 41 (and 42 in the scan), v2c decode refusals
268, v2a rewrites of a record carrying schema-B tags 34, schema-B rewrites dropping reserved
tag 3 169, starved continuations 17 (searches) and 34 (scan).

### Findings and triage

1. **Real SDK hole: the A3 kind marker is missing when a new collection is written through
   `at_mut` (medium; not fixed, pinned).** Found by the scan (witness 5), minimal plan: v2a
   creates g0 (tags 1-5, no tag-8 marker); upgrade to v2b; `add_flag(g0)` writes the `flags`
   entries through `at_mut`, which never rewrites the record, so no marker lands; downgrade to
   v2a; `remove_group(g0)` (or `remove_step`) passes the A3 check and leaves 4 keys (flag entry,
   index, slot, header) behind; a new g0 created in v2a shows `f0` again once v2b is back. The
   spec's "the kind marker closes this" is false for a collection added to an existing record
   type. The mock reproduces it too (`a3::hazard_a3_marker_missing_when_the_new_collection_is_written_through_at_mut`
   in `state_v2_verify.rs`, passes while the hole exists); the sim's
   `a3_marker_missing_after_at_mut_write_orphans_a_new_collection` and the saved scan case
   `a3_orphan_replay` pin it on the runtime and flip to asserting the refusal once fixed. Not
   fixed on this branch because each fix changes a stated cost or leaves part of the hole:
   (a) at flush, read every record whose children `at_mut` wrote and append missing markers:
   sound, +1 record read on every `at_mut` write path (the read D9 removed; DAO `vote` reads
   its 2,000-byte description again); (b) the same read only on the cold path where the
   collection's header was absent before the call (Map, Set, Vector already read it; Lazy needs
   one `storage_has_key`): near-free, but `LookupMap`/`LookupSet` have no header, so the hole
   stays for them; (c) schema-level: a collection field added to an existing record type must
   be declared as added, and only it pays (a); `storage diff` enforces the declaration. The
   spec's A3 "Not covered" list now names it. Decision needed.
2. **Re-tagged field (expected hazard, confirmed).** Same-length re-use is silent in both
   directions: v2a `locked = 5` reads as v2b `claimable = 5`; v2b `claimable = 42` reads back as
   v2a `locked = 42` (`retag_both_directions`). The tag-level model predicts the bytes exactly;
   the intent check fails with cause "tag 4 re-used" and a `Known` keyed by that cause covers
   it. A different-length retype would fail every decode instead (not built). Its breadth
   (512/512 in the v2b search) is the fixtures', which carry v2a-written tag-4 records;
   `assert_known_is_narrow` needs a second failure of the same check, which a correct build
   cannot produce, so only the positive control and a clean v2b-only control were run
   (`retag_hazard_is_caught`). Triage: by design of the experiment; `--no-abi` skips the
   `storage diff` that would refuse it.
3. **`required` added later (expected, documented).** Under v2c an old record fails `get`,
   `get_mut`, `remove_step` and `remove` with `Missing { tag: 9 }`, while `at_mut` writes below
   it (`add_member`, `add_flag`, `remove_member`) still land, so the record keeps growing and can
   never be removed until a rollback (`required_tag_on_old_records`). Not a bug: the spec forbids
   the change and `storage diff` refuses it.
4. **Detached continuations meeting refusals (contract design, Known).** After a redeploy the
   next `clear_loop` link can meet the A3 refusal, v2c's decode refusal or the clearing guard;
   it fails atomically with nobody told (sim built-in "unhandled failure", 3 search cases, 42
   scan variants). Keyed by `Known::when` on the refusal message at each continuation site.
   SDK behaviour is correct; a production driver should surface the stop.
5. **Harness/contract bugs fixed during the run:** my first driver attached a fixed 40 Tgas to a
   continuation from a 40 Tgas receipt, so every second link ran out of gas (fixed: forward the
   remaining gas less 10 Tgas, stop below 20); the orphan model first expected one key per Set
   entry instead of three (entry, index, slot) plus header.

Everything else held on every case: one side per item through every upgrade; no v1 key after a
drain; seq gapless through deploys, downgrades and continuations; records byte-exact through
v2b -> v2a -> v2b (v2a's `get_mut` keeps `level`/`label`; tag 3 is dropped on the first
schema-B rewrite, intentionally); `remove_step` results matched the budget rule on every call,
including resumes under another schema and after starved links; `storage_usage` equalled
account + code + Σ(key + value + 40) after every block; the items subtree matched the spec's
staked-bytes formula. A drained contract with the wrapper off equals a fresh v3 contract with
the same 12 items in every items key and byte, and its `storage_usage` is 18 bytes higher, all
in the root record: tag 1 keeps the stale v1 `IterableMap` bytes (prefixes `iv`/`im`)
(`drained_contract_matches_a_fresh_v2_one`).

### Fidelity and what the atomicity result means

A receipt that runs out of gas keeps no state change; injected `OutOfGas` runs the receipt with
1 gas and burns almost nothing (fidelity.md, "A fault rewrites a receipt between blocks"), and
explored gas below the need fails where a node would. So "a call that ran out of gas in the
middle of `migrate_batch` or `remove_step` leaves a state the next call completes" is nearcore's
atomicity, not an SDK property; it held trivially. What the sim judged that the mock harnesses
do not run: real wasm builds of five versions deployed over each other's state, deploy and
`migrate` in one transaction and the deploy-without-migrate window (719 refused calls),
continuations starved or landing on other code, overlapping calls in one block, and the
runtime's storage staking. The A3 hole itself is reachable in the mock (the pin above); the
mock suites never tried a record written by one schema and extended by another through
`at_mut`. One shard and a static gas price (fidelity.md, "Limits") do not matter here: every
receipt is on one account.

### Not done

No fix for finding 1 (decision above). No variant with a different-length re-tag, a `Lazy` or
`Vector` field added in a later version, or a v3 deployed before the drain finished (the op
skips it: the documented precondition). Searches rarely land OutOfGas (fault draws fit only
continuations); the scan covers every continuation of its 8 witnesses.

## Sim harness port to primitives main (Oct 20)

**Question.** Both sim harnesses (`proto-sim-search/`, `proto-sim-upgrade/`) were written against near-contract-sim b328d067, the old explore layer. Do they port to main @ 8e8642db ("One crate: primitives replaces main (#83)"), and do the results hold once the sim team's triage (`.c1-work/q3/storage-v2-triage.md`) is applied? **Answer: they port, and the results hold.** No new SDK finding. The A3 orphan replay still finds the hole on ec29bf6f. Main's scan, which now includes type-max argument edges, turned up two harness model bugs and one witness that should have been marked refused. All three are fixed. The branch is `proto/storage-v2-sim-port`, and the harnesses depend on `../../../near-contract-sim-main`, a worktree at 8e8642db (setup in each README).

**API diffs (old -> main).**

- The module is still `near_contract_sim::explore`. `Report` -> `Audit`. `report.coverage().assert_reached(m)` -> `audit.assert_reached(m)`, and `Audit::reached(m)` counts distinct cases.
- `harness.scan_each(&plans, Faults)` -> `harness.scan(&plans).faults(Faults::default()).run()`.
- The gas walk (a hand loop over `Run::outcomes()`) -> `harness.curve(&plan, step, "caller gas").run()`, one curve per step.
- `harness.search(...)` + `.cases(n)` -> `harness.search(strategy).cases(n).run()`. Guidance growth and max-gas growth are gone: every case is a drawn plan.
- `Known`, `Known::when`, `out_of_scope`, `assert_no_new_findings`, `Seen`, `Report::elapsed`, `rerun` and `coverage()` are all gone. On main a finding is never excused. The harnesses now assert each expected finding by check name and `Finding.cause`, and any other finding fails the test. `out_of_scope(site, why)` -> `.excuse(todo_id, why)`, which closes a coverage to-do item but never a finding. `.audits([APP])` limits coverage to-dos to the audited account.
- `Fail` gained `Fail::not_judged(why)` and `Fail::idle(why)`. `per_op_for(pred, name, f)`, which existed already, now replaces the harness's `per_op` checks so a check runs only on the ops it is about. Checks return `Fail::new(..)` with `.key(..)` and `.cause(..)`, or one of the two new kinds.
- `Plan::new` / old replay JSON (`gas: null`, `inject: null`) -> `Plan::load(path)` with the plan under `"plan"`. `inject` is a `Vec`, so the A3 replay file was converted (`tests/found/a3-orphaned-flags.json`).
- A written witness whose transaction fails is now a harness error unless it is marked `Plan::refused()`.
- `plans(ops).repeats(..)` is unchanged. The harnesses now set it to `false`.
- `Audit.judged` is crate-private at 8e8642db and not printed. The harnesses read it from `audit.report().json()["judged"]`.

**Triage items, as applied.**

1. **Not judged on overlap.** `remove_step only shrinks the record` is a `per_op_for(is_remove_step, ..)` and returns `Fail::not_judged("overlapped: ...")` when the op is not isolated. The old version also counted every non-RemoveStep op as ok, which the triage didn't mention. `a failed op leaves no trace` was dropped: the at-rest model replay already judges every receipt, overlapped or not. Counts are below. In the search harness, not judged is 0 because every gap is `Gap::Settle` and main has no growth, so ops never overlap. The upgrade harness uses default gaps, so its ops do overlap. Its one per-op check (`failures are documented ones`) reads only the op's own receipts, and that holds under overlap. Its other checks are at-rest checks over a model folded by `seq`. So 0 not judged is honest there too.
2. **Faults drawn and landed.** Main at 8e8642db prints neither, so the harness computes them. It wraps the plan strategy to keep each distinct drawn plan. After the search, it reruns each faulted plan through `Harness::run`, and a fault has landed if `Run::plan()` still carries it. Faults never reach a transaction's first receipt (faults.md), so most draws fit nothing.
3. **`.repeats(false)`, and the reducer `Blocks(0)` bug.** The reducer bug is in `shrink.rs` `smaller()`: `p.steps.drain(block)` leaves the earlier step's `then`. It is fixed on origin/fix/open-gaps 43041c11. It did not bite the search, which drew no `Blocks(0)`, had nothing to shrink, and judged 0 not judged. The upgrade harness draws `Blocks(0)` through default gaps, so its shrunk plans may carry a stale `then`. That would show only in shrunk witnesses of expected findings. Its model folds by `seq`, so a stale `then` changes no verdict.
4. **Coverage via `Audit`.** The search asserts `Audit::reached` for all 29 methods, and the upgrade search for `insert` and `remove`. Both print the distinct-case count per method. The in-check tally stays as `probe: behaviour classes (never fails; counts include reruns)`, an `at_end` check whose name says what it is. Its counts span every execution: shrink candidates, fault baselines, case 0's rerun and the faults line's reruns.
5. **Padding checks.** These are now labelled as non-SDK and kept out of the SDK property table: `runtime: storage_usage is base + key + value + 40 per key` (nearcore's accounting rule, which the triage missed), `harness: seq has no gap`, and the probe. The positive control is kept: `positive_control_orphan_key_is_found` plants an orphan with a harness-only op (`PlantOrphan`, `set_state` in `submit`) and asserts the model check finds it (`accounts[alice.near].log/0007000000: EXTRA`). It had to move from the fixture into the plan, because on open-gaps a check failing on the fixture is a harness error. A step that submits no transaction gets no rest judgement, so a `Read` follows the plant.
6. **opt-level 2.** `[profile.dev.package."*"] opt-level = 2, debug = false` is now in both `sim/Cargo.toml`. The first build compiled nearcore at opt-level 2 in 8 min 56 s, with mbx reporting 1,333 shared-cache hits and 196 misses. In an A/B over 128 fixed drawn plans, two interleaved rounds on a machine at load 48 to 97, search took 338 and 371 ms per case at opt-level 0 against 127 and 124 ms at opt-level 2. Upgrade took 479 and 620 ms against 189 and 214 ms. That is 2.5 to 2.9x faster (`results/opt_level_ab_20261020b.txt`).

**Runs (main @ 8e8642db, opt-level 2, `*_20261020b.txt`).**

| Campaign | Before (b328d067) | After (main) |
| --- | --- | --- |
| Search smoke + positive control | clean; control caught | clean in 0.3 s; control caught |
| Search, mixed gas, seed 0x20261020 | 2,000 cases + 368 grown + 366 max-gas; 0.22 s/case; per-op checks 55,071 ok, of which 38,050 overlapped and were counted ok; faults: 2 injected, 406 fit nothing | 2,000 cases, clean, 0 findings, all 29 methods reached; model check 14,905 ok, 0 not judged; `remove_step` check 1,763 ok, 0 not judged; 0 incomplete; 57 ms/case excluding shrinking, 114 s wall; **faults: 502 drawn, 2 landed** |
| Search, wide gas, seed 0x20261021 | 2,000 cases; 0.04 to 0.05 s/case | clean; 15,200 / 1,733 ok, 0 not judged; 14 ms/case, 28 s wall; **faults: 477 drawn, 1 landed** |
| Search scan of 2 witnesses | `scan_each`: 477 variants, 48 injected, 56 to 58 s | `scan`: 734 cases, 0 findings, 19,245 / 1,980 ok, 0 not judged, 106 s (after the two model fixes below) |
| Search gas | gas walk: 1,228 runs, 714 deaths mid-flush, 60 to 67 s | caller-gas curves, 13 steps: 101 cases, 2.5 s, clean |
| Upgrade hand tests | 9 pass | 9 pass; `a3_orphan_replay` still finds the orphan with cause "no collection outlives its record" |
| Upgrade search from v2a, seed 0x102 | 512 cases; 177 ms/case; 22 injected, 116 drew a fault nothing fits; retag `Known` suppressed 115 cases | 2,000 cases; only the 2 expected findings: retag (249 cases), and unhandled failure at `clear_loop` after a v2c redeploy (4 cases); `no collection outlives its record` 14,246 ok; 0 not judged, 0 incomplete; all 16 methods reached; 152 ms/case excluding shrinking, 20 min wall (shrinking the retag finding took most of it); **faults: 504 drawn, 83 landed** |
| Upgrade search from v1, seed 0x101 | 512 cases; 145 ms/case; 0 injected, 125 fit nothing; 29 every op skipped | 2,000 cases; only retag (18 cases); 13,235 ok on each SDK check, 0 not judged; 141 incomplete, all "no op ran: every op skipped" (checked on the first 300 cases: 21 of 21) — the generator's shape, as before; 235 ms/case, 9 min wall; **faults: 520 drawn, 0 landed** |
| Upgrade search from v2b, seed 0x103 | 512 cases; 200 ms/case; 17 injected, 120 fit nothing; `Known`s suppressed retag 512, continuation 3, A3 0 of 512 | 2,000 cases; expected findings only: retag (all 2,000 cases, from the v2a-written fixture), **the A3 orphan (1 case: "g1 was removed through v2a and left its flags behind")**, and unhandled failure at `clear_loop` (5 + 9 cases); 0 not judged, 0 incomplete; 364 ms/case, 15 min wall; **faults: 495 drawn, 79 landed** |
| Upgrade OutOfGas scan, 8 witnesses | 472 variants / 480 cases, 136 injected, 45 s; `Known`s suppressed retag, A3, continuations | 638 variants / 646 cases (136 forcing a fault, 502 at caller-gas and argument edges), 158 s; expected findings only: retag 311 cases, A3 56 + 1 (at `rounds = u32::MAX`), unhandled `clear_loop` 50 + 4 (at `to = V2c`); 0 not judged, 0 incomplete. Five methods are unreached to-dos because no witness calls them (`remove`, `remove_group`, `remove_member`, `set_field4`, `set_weight`). The searches reach them |

The upgrade per-case times ran at load 80 to 150, against a lightly loaded machine for the b328d067 runs, so the two columns can't be compared. The A/B above is the fair one. **What changed in the results:** no SDK verdict changed. The searches still hold every SDK property, and the upgrade campaigns still produce exactly the three expected kinds. One thing is new. At 2,000 cases the v2b search reaches the A3 orphan by itself. At 512 cases the A3 `Known` had suppressed 0 cases, so before this only the scan's witnesses reached it. Judged and not-judged counts are now honest: 0 not judged everywhere, where the old per-op checks had counted 38,050 overlapped ops as ok. Every incomplete case is one where every op skipped: a v1 world with no Upgrade drawn, so every group op skips. That is 141 of 2,000 cases (7%), against 29 of 512 before. All 21 of the 21 incomplete cases in a 300-case sample with the same seed gave that reason.

Opt-level explains most of the search's per-case drop, as the triage said. Wide gas, which had almost no growth before (2,005 runs for 2,000 cases), went from 0.045 to 0.014 s, which is 3.2x and matches the 2.8x A/B on identical plans. Mixed gas went from 0.22 to 0.057 s, which is 3.9x, so removing growth adds about 1.4x on top. Mixed is still about 4x slower than wide, as before (4.9x), so the gap comes from the gas strategy, not growth. Both columns are load-confounded, and the A/B is the clean number.

**New findings, triaged.** No SDK bug.

1. **Harness model bug (search), fixed in a85afe2a.** `Model::exec` ran a receipt's method before looking at its outcome. For `acct_bulk { n: u32::MAX }`, which main's scan draws as an argument edge, the receipt died at the gas ceiling, but the model still looped 4 billion times with an O(n) find, and the scan hung for 12 h. The model now classifies the outcome first: gas failures change nothing, and any other failure is an error.
2. **Harness/test-contract fidelity (search), fixed in 68b828c6.** The same edge with `blob = u32::MAX` makes the contract's `vec![b; 4 GiB]` panic on wasm32 with `capacity overflow`, while the native model allocated 4 GiB and predicted success. The model now predicts `Panic("capacity overflow")` above `i32::MAX`. This is the test contract's allocation, not the SDK (`scan_20261020b_before_capacity_model.txt` keeps the evidence).
3. **Witness bug (upgrade), fixed in c3f92d62.** Scan witness 3 runs ClearLoop g0, Upgrade V2b, AddMember g0, RemoveStep g0. Its `add_member` is refused once the chain has cleared g0 ("no such group"). Main calls a failing written witness a harness error, so the witness is now `.refused()`.
4. **Sim behaviour worth knowing.** A step that submits no transaction gets no block and no rest judgement, so a harness-only state edit must be followed by a real call to be checked.

**What open-gaps (origin/fix/open-gaps 43041c11, not used here) will change.** It prints a not-judged summary line, auto-not-judges an overlapped op's before/world reads, makes a check failing on the fixture a harness error, and fixes the reducer. The local unpushed commits in `~/Developer/ncs-open-gaps` add the `faults: N drawn, M landed` summary (51b71786), which can replace this harness's computed line.

**Disagreements with the triage.**

- The triage asked for faults "as the report prints it". 8e8642db prints nothing, so the line is computed harness-side, with the same definition as open-gaps' 51b71786.
- The `storage_usage` checks are runtime properties too, so they are labelled and kept out of the SDK table.
- Curves don't replace the walk's coverage. At 8e8642db the caller-gas ladder starts at 1 Tgas and climbs fourfold (`curves.rs` `ladder`), with at most 8 refinements, so it finds the region where a step starts passing but doesn't sweep death positions the way the walk did (714 mid-flush deaths). Six of the 13 steps (3 to 7, and 12) need under 1 Tgas, so their curves ran 3 passing cases each and never saw a failure. Mid-op exhaustion now comes from the searches' gas strategies and the scan's least-caller-gas edges. F3 position coverage stays with the mock. A walk below 1 Tgas would need a pinned `StepGas::Exactly` sweep in the harness. That is worth raising with the sim team.
