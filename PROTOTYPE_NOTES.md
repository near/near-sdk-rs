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
- No migration path for records written with the old `00 00` prefix. The prototype never shipped, so such a record now fails closed with `TagZero`.
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
