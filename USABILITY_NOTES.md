# `near_sdk::state` usability notes

Three contracts written against `near_sdk::state` as a contract developer would, using only the
module's rustdoc/source doc comments, `tests/state_v2_exchange.rs`, and ordinary near-sdk tests
as references:

- `near-sdk/tests/state_v2_ft.rs`: NEP-141 + NEP-145 fungible token (14 tests)
- `near-sdk/tests/state_v2_nft.rs`: NEP-171 + 178 approvals + 181 enumeration (15 tests)
- `near-sdk/tests/state_v2_dao.rs`: members, proposals with votes, inline config (13 tests)

Headline: the FT and NFT compiled on the first attempt, and every op count I measured matched
the per-method doc comments once I read them carefully. The friction is concentrated in five
places:

1. A derive-generated type name (`{Record}View`) that collides with the obvious DTO name.
2. `IterMap::page` being unusable for any record that holds a collection.
3. There is no way to write to a nested collection without reading its parent record.
4. A post-monomorphization check (`Map<K, Collection>`) that `cargo check` and
   rust-analyzer never report.
5. The attributes themselves (`#[near(storage)]`, `#[key]`, `layout = v2`) have no rustdoc
   at all.

---

## 1. Compile errors I hit (ranked by cost)

### 1.1 `#[near(storage)] struct Proposal` silently defines `ProposalView` (cost: highest, 36 errors)

The code:

```rust
#[near(storage)]
pub struct Proposal { #[key(1, required)] proposer: AccountId, #[key(2)] description: String,
                      #[key(3)] votes: IterMap<AccountId, Vote> }

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
pub struct ProposalView {          // my JSON DTO for the "proposal with tally" view
    pub id: ProposalId, pub proposer: AccountId, pub description: String,
    pub yes: u32, pub no: u32, pub votes: Vec<(AccountId, Vote)>,
}
```

The output was `could not compile ... due to 36 previous errors`. The first error is the right one:

```
error[E0428]: the name `ProposalView` is defined multiple times
  --> near-sdk/tests/state_v2_dao.rs:60:1
   |
48 | #[near(storage)]
   | ---------------- previous definition of the type `ProposalView` here
...
60 | pub struct ProposalView {
   | ^^^^^^^^^^^^^^^^^^^^^^^ `ProposalView` redefined here
```

After it come 35 errors that all describe the generated type, not mine:

```
error[E0726]: implicit elided lifetime not allowed here          (x3)
error[E0106]: missing lifetime specifier ... consider using the `'de` lifetime
error[E0609]: no field `yes` on type `&ProposalView<'_>`
   = note: available fields are: `proposer`, `description`, `votes`, `__m`
error[E0369]: binary operation `==` cannot be applied to type `near_sdk::state::UseGetForScalars`   (x4)
error[E0277]: the trait bound `near_sdk::state::UseGetForScalars: serde::Serialize` is not satisfied
error: lifetime may not live long enough
   = note: the struct `ProposalView<'a>` is invariant over the parameter `'a`
error[E0308]: mismatched types   (x6)
...
```

Nothing in the rustdoc says the derive emits a public type named `<Name>View`. `Record::View<'a>`
is documented as an associated type, not as a named struct in my module. `ProposalView`,
`TokenView`, and `AccountView` are exactly the names people give their JSON view DTOs in NEAR
contracts (`near-contract-standards` names its NFT JSON type `Token`, which is the other natural
choice and would collide with a `#[near(storage)] struct Token` just as easily via `TokenView`).
**Fix:** I renamed my DTO to `ProposalJson`.
**Suggestion:** don't put a generated name into the user's namespace, or at least pick one
nobody uses (`ProposalAt`, `__ProposalView`, or a hidden module), allow
`#[near(storage, view = Name)]`, and document it. If the derive also emits Default and a
Removable impl, document those too (see 4.1).

### 1.2 `IterMap::page` needs `V: Clone`, and a record with a collection can never be `Clone` (cost: 2 rounds)

```rust
let page = self.proposals.page(after.as_ref(), limit);   // proposals: IterMap<u64, Proposal>
```
```
error[E0277]: the trait bound `Proposal: Clone` is not satisfied
   --> near-sdk/tests/state_v2_dao.rs:185:35
185 |         let page = self.proposals.page(after.as_ref(), limit);
    |                                   ^^^^ unsatisfied trait bound
note: required by a bound in `IterMap::<K, V>::page`
507 |         V: Clone,
```

The obvious next step, `#[derive(Clone)]` on the record:

```
error[E0277]: the trait bound `IterMap<near_sdk::AccountId, Vote>: Clone` is not satisfied
56 |     votes: IterMap<AccountId, Vote>,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Clone` is not implemented for `IterMap<...>`
```

**Fix:** `keys_page` + `get` per key, then rebuild a `Page` by hand. The `keys_page` doc does
say "this is the path for record-valued maps, which `page` cannot clone". I found that only
after the second error. It belongs on `page` itself, and ideally on the `Clone` error
(`#[diagnostic::on_unimplemented]` can't be put on `Clone`, but `page` could take a mapping
closure: `page_map(after, limit, |k, v| -> T)`).

### 1.3 `Map<K, IterSet<_>>` is accepted by the declaration and by `cargo check` (cost: would be high in practice)

I hit this one on purpose. Before reading that a `Map` value can't be a collection, a
`Map<AccountId, IterSet<TokenId>>` for "tokens per owner" is the first thing I'd write.

```rust
#[key(2)] sets: Map<u32, IterSet<u32>>,
...
pub fn b(&mut self) { self.sets.get_mut(&1); }
```

- The struct declaration compiles.
- `cargo check` passes, so rust-analyzer shows nothing.
- `cargo test` also passes, as long as no test calls the method: an uncalled contract method in
  a test binary is never monomorphized, so the `const` assert never runs. A wasm build would
  hit it because the wrapper calls every method.
- When it does fire, it has no span in user code:

```
error[E0080]: evaluation panicked: a map value cannot itself be a collection; use GroupedMap
   --> .../rustlib/src/rust/library/core/src/panic.rs:62:9
    |   evaluation of `near_sdk::state::Map::<u32, near_sdk::state::IterSet<u32>>::CHECK` failed here
   ::: near-sdk/src/state/collections.rs:180:23
note: the above error was encountered while instantiating `fn near_sdk::state::Map::<u32, IterSet<u32>>::entry_path`
```

The message itself is good ("use GroupedMap"). Where it fires is the problem. **Suggestion:**
reject this in the derive, which sees `Map<_, IterSet<_>>` syntactically for the common
names, or bound `Map`'s `V` on a sealed `Inline` marker trait so it's an ordinary
trait error at the field.

### 1.4 Writing through `at()` (cost: low, but the error doesn't help)

```rust
self.proposals.at(&id).votes.insert(env::predecessor_account_id(), vote);
```
```
error[E0599]: no method named `insert` found for struct `At<'a, C>` in the current scope
help: there is a method `iter` with a similar name, but with different arguments
```

The suggestion (`iter`) is noise. What I needed was "views are read-only; to write to a
nested collection, `get_mut` the record". No `at_mut` exists (see 3.2).

### 1.5 Scalar through a view (cost: low; the error is good)

```rust
self.proposals.at(&id).proposer.clone()
```
```
error[E0308]: mismatched types
194 |         self.proposals.at(&id).proposer.clone()
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `AccountId`, found `UseGetForScalars`
```

The type name does carry the hint, as the doc comment intends. It works.

### 1.6 `entry().or_default()` on a record with a `required` field (cost: low)

```
error[E0277]: the trait bound `Req: Default` is not satisfied
 30 |         let _ = &self.reqs.entry(1).or_default().owner;
note: required by a bound in `MapEntry::<'a, K, V>::or_default`
```

The error is clear, but it's surprising that adding `required` to one field removes the
record's `Default`. The `FieldDefault` diagnostic explains it for fields; nothing explains it
for the record.

### 1.7 `remove` on a record that holds a `Map` (cost: low; the error is decent)

```
error[E0277]: the trait bound `HasMap: Removable` is not satisfied
 36 |         self.hm.remove(&1);
help: the trait `Removable` is conditionally implemented for `HasMap`
 11 | #[near(storage)]
    | unsatisfied requirement introduced here: `near_sdk::state::Map<u32, u32>: Removable`
```

It names the offending field type, which is better than I expected. It doesn't say *why* `Map`
isn't removable (it can't enumerate its keys) or suggest `IterMap`. An
`on_unimplemented` on `Removable` would fix that: "`Map` cannot be cleared because it does
not track its keys; use `IterMap`/`IterSet`, or `discard` the record and accept the orphans".

**Not compile errors, but runtime panics that cost a round each:**
- Setting a nested `Lazy` before its record is inserted. See 2.8. The message is excellent.
- My `nft_transfer` op-count guess was off. See 3.5.

---

## 2. Things I guessed

Each entry gives what I assumed and whether a test confirmed it.

1. **Record vs Borsh leaf as a map value.** `FtAccount { balance, storage_deposit }` could be
   a `#[near(storage)]` record or a plain Borsh struct. Nothing says which to prefer. I
   guessed a record, because it can gain fields later and a Borsh leaf can't. Cost: 2 bytes of
   tag+len per field. *Unconfirmed as a recommendation*; I'd like a sentence in the docs.
2. **`#[init]` without `.bind()` is fine if the body doesn't touch collections.** (`Dao::new`
   only sets scalars and an inline record.) *Confirmed*: the DAO tests pass.
3. **`get` then `get_mut`/`remove` on the same key in one call is one read.** *Confirmed*:
   `storage_unregister` does `get` then `remove` and costs `(2, 0, 1)`, root plus one record read.
4. **`contains_key` warms the cache for a following `get_mut`.** *Refuted*: it's a
   `storage_has_key` with no bytes, so the later `get_mut` reads anyway.
   `ft_transfer` = `(3, 2, 0)` + 1 has_key. "Check the receiver first so a failed transfer
   touches nothing" therefore costs an extra host call. Not a bug, but the cost is invisible
   unless you read both doc lines together.
5. **Two `get_mut`s on different root fields alive at once compile.** (`vote` holds `&mut Member`
   and `&mut Proposal` together.) *Confirmed*: normal disjoint field borrows.
6. **Scalars of a record returned by `remove` are readable** ("detached: its collections panic
   on use"). *Confirmed*: `nft_burn` reads `token.owner_id` after `remove`, and
   `storage_unregister` reads `removed.balance`.
7. **`remove` on a record clears nested `IterMap`, `IterSet`, and `Lazy`.** *Confirmed* by store
   key-set equality before and after: an NFT burn with 3 approvals plus `Lazy` metadata, DAO
   `finalize`, and DAO `remove_member` all leave no orphaned keys.
8. **A nested `Lazy` can be filled before the record is inserted.** *Refuted*, at runtime:
   `state: collection is not bound to a path; place its record with 'insert' or 'or_default()'
   first ...`. I had to insert, then `get_mut(&id).metadata.set(..)`.
9. **A panic mid-method leaves nothing behind for the next call in the harness.** *Confirmed*
   (`failed_transfer_leaves_state_untouched`): the next `begin_call` drops the cache. On chain
   the receipt aborts anyway.
10. **Enumeration order after remove.** I assumed swap-remove, as in v1 `IterableSet`, because
    the doc mentions "the last slot". *Confirmed*: `[t0,t1,t2,t3]`, transfer `t1`, gives
    `[t0,t3,t2]`. Anyone porting a v1 `TreeMap`-backed or append-order list will be surprised;
    the type docs should say "order is not insertion order after a remove".
11. **A cursor page whose cursor key was removed.** The doc says it panics. *Confirmed*. In a
    public view such as `nft_token_ids_for_owner`, that means a frontend paging while a
    transfer happens gets a failed view call, where offset pagination would only skip or repeat
    an item. I shipped offset pagination for NEP-181 for that reason.
12. **Cursor page that starts at the last key.** I assumed an empty page with `next: None`.
    *Confirmed*.
13. **Storage bond for NEP-145.** v1 FT contracts hard-code 125 bytes. There's no way to compute
    a v2 entry's size from the schema, and the counting mock doesn't track `storage_usage`. I
    switched to `use_env_host()` and measured: **152 bytes** for one registration of a
    53-character account (76-byte key + 36-byte record + 40 bytes of overhead), about 27 bytes
    more than v1's `LookupMap<AccountId, u128>`. A 64-character account comes to about 163. I set
    the bond to 200. A `Map::<K, V>::max_entry_bytes()` const, or a doc table, would save
    everyone from measuring.
14. **Whether `finalize` with many voters fits in gas.** Untested. `IterMap::clear_some` exists
    for chunked cleanup, but `finalize` has to be atomic for the tally to mean anything. I
    guessed a real DAO needs either a voter cap or a two-phase finalize. The module gives no
    guidance.
15. **What `#[near(storage)]` emits.** I learned that it emits `Default` (unless a field is
    `required` or the user derives one), a `Removable` impl, and a `<Name>View` type by
    reading the header comment of `near-sdk-macros/src/core_impl/code_generator/storage_v2.rs`.
    None of that is in rustdoc. The exchange test only hints at it ("the derive emits
    `Default`").

---

## 3. Awkward code

### 3.1 Minting a token with nested `Lazy` metadata takes two steps

```rust
// v2 (what I had to write)
self.tokens.insert(id.clone(), Token { owner_id, next_approval_id: 0,
                                       approvals: Default::default(), metadata: Default::default() });
self.tokens.get_mut(&id).expect("just inserted").metadata.set(metadata);
```

What I wanted is `Token { .., metadata: Lazy::new(metadata) }`, with the value held until the
record is bound. This isn't worse than v1 (where you'd build a `LazyOption` with a hand-made
prefix), but it is the one place where the v2 struct literal doesn't describe the value.
The extra `get_mut` costs nothing, because the record is already cached.

### 3.2 No `at_mut`: writing to a nested collection reads the whole parent record

`vote` must reach `proposal.votes`. The only way to write there is
`self.proposals.get_mut(&id)`, which reads and decodes the `Proposal` record, description
included:

```
vote rwd=(7, 8, 0) has_key=1 read_bytes=2074
  Read len=27 value=2019      <- the Proposal record, 2000-byte description, read to insert a vote
```

`at(&id)` reaches the same path without reading, but it's read-only. A v1 developer would
have used `LookupMap<(u64, AccountId), Vote>` and read nothing. In v2 I can do that too, but
then `proposals.remove` no longer clears the votes, and the cascade is the reason to nest.
The workaround is `description: Lazy<String>`, which puts me back in 3.1.
**Suggestion:** `map.at_mut(&k)` returning a mutable view whose collection fields are `&mut`
handles and whose scalars are unavailable, mirroring `at`.

### 3.3 Paging records

```rust
pub fn proposals(&self, after: Option<ProposalId>, limit: u32) -> Page<ProposalId, ProposalJson> {
    // `page` needs `V: Clone`, which a record holding a collection can never be.
    let page = self.proposals.keys_page(after.as_ref(), limit);
    Page {
        items: page.keys.into_iter().map(|id| (id, self.get_proposal(id).unwrap())).collect(),
        next: page.next,
    }
}
```

This works and costs the same as `page` would. But every contract will write exactly this
function. A `page_with(after, limit, |k, &v| T) -> Page<K, T>` would remove it.

### 3.4 NEP-181 offset pagination by hand

NEP-181 uses `from_index: U128, limit: u64`. The module only has cursor paging, so:

```rust
let group = self.tokens_per_owner.group(&account_id);
let start = u32::try_from(from_index.map_or(0, |i| i.0)).unwrap_or(u32::MAX);
let limit = u32::try_from(limit.unwrap_or(50)).unwrap_or(u32::MAX);
let end = start.saturating_add(limit).min(group.len());
(start..end).map(|i| group.key_at(i).expect("dense")).map(|id| ...).collect()
```

v1's `IterableSet::iter().skip(n).take(m)` is shorter, though it reads more. Since `key_at`
already makes offsets O(1), a `keys_range(start, limit)` would make the standard NEP shape a
one-liner.

### 3.5 "Grouped set" via `GroupedMap<G, K, ()>`, and `remove` vs `discard`

Per-owner token lists have to be `GroupedMap<AccountId, TokenId, ()>`. A `Map` value can't be a
collection, and `IterSet` has no `key_at`/`keys_page`, so `Map<_, Record { tokens: IterSet }>`
can't paginate. With `()` values:

- `group_mut(&owner).remove(&id)` reads the `()` entry before deleting it. That's 1 wasted read
  per transfer, and the reason my guess of `nft_transfer = (7,7,3)` came out at `(9,8,3)`.
- `discard` gives `(8,8,3)`.

`IterSet::remove` already calls `discard` internally, so a real `GroupedSet` would make this
free. `IterMap::remove` could also skip the read when `V` is zero-sized.

### 3.6 Double-vote check is two ops where one would do

```rust
require!(!proposal.votes.contains_key(&voter), "already voted");   // has_key
proposal.votes.insert(voter, vote);                                // index read, header read, 4 writes
```

`IterMap::insert` already reads the index to decide new-or-existing. An
`insert_new(k, v) -> bool` (or `entry` returning `Vacant`/`Occupied`, like `std`) would make
"one vote per member" a single call and save the has_key. Today, `IterMap::insert` on a leaf
silently overwrites, which is exactly the double-vote bug.

### 3.7 Back-reference cleanup is linear, and the cost isn't obvious until measured

`remove_member` has to find a member's votes, which forces a `votes: IterSet<ProposalId>`
back-reference on `Member`. Measured costs:

- `finalize` with 5 votes: `(33, 0, 44)`.
- `remove_member` owning one proposal with 4 votes: `(29, 3, 40)`.

That's about 9 host ops per vote. This is inherent to the design, not the module, and v1 would
be the same or worse. I note it because `remove` making the *forward* cascade free makes the
*backward* references look free too, and they aren't.

### 3.8 Test harness boilerplate

Each test file copies about 30 lines of `ctx` / `init` / `call` / `view` from the exchange
test. The `testing` module the harness relies on is `#[doc(hidden)]`. Two harness traps:

- `testing_env!` resets `storage_usage` from the builder, so a before/after measurement has to
  be taken after the context switch. I got a subtraction overflow first.
- `use_mock()` and `use_env_host()` are mutually exclusive, and only the env host tracks
  `storage_usage`, while only the mock gives receipts.

A public `state::testing::Harness<T: Root>` with `call(who, deposit, f)` and `view(f)` would
pay for itself.

### 3.9 Where it was *not* awkward

`ft_transfer` is line-for-line what I'd write with v1 `LookupMap`. `storage_unregister`'s
`get` → check → `remove` is the natural shape and costs one read.

---

## 4. Naming and discoverability

1. **The attributes have no rustdoc.** `#[near(storage)]`, `#[key(n)]`,
   `#[key(n, required)]`, `reserved(..)`, and `contract_state(layout = v2, root = ..)` appear
   nowhere in `near_sdk`'s docs, and `state/mod.rs` has no module-level doc. The exchange test
   is the only usage guide. Missing in particular: what the derive emits (`Default`, `Removable`,
   `<Name>View`), when `Default` is *not* emitted, and that tags must never be reused.
2. **`<Name>View`**: see 1.1. Rename it or hide it.
3. **`at` vs `get`**: once learned, the split makes sense ("`at` walks the path, `get` reads").
   But `at` reads as positional (`Vec::at`, C++ `at(i)`). Nothing about the name says
   "read-only, reads nothing, no scalars". I'd call it `path(&k)`, `under(&k)`, or `nested(&k)`,
   with `nested_mut` alongside.
4. **`Vec`** shadows `std::vec::Vec`. I deliberately never imported it, since my DTOs all use
   `Vec<(AccountId, Vote)>`. `use near_sdk::state::*` would silently break every `Vec` in the
   file. I'd call it `Slots`, `DenseVec`, or `StateVec`.
5. **`Map` vs `IterMap`**: v1's `LookupMap` name warned you it can't iterate. `Map` doesn't, and
   it also hides "not `Removable`, so a record containing one can't be removed". I'd keep
   v1's `LookupMap`/`IterableMap` naming, or put the consequence in `Map`'s type doc. Currently
   that doc is one line about key layout.
6. **`Lazy`** clashes with `near_sdk::store::Lazy` (the derive even notes it can't tell them
   apart by name) and with `std::sync::LazyLock`/`once_cell::Lazy`, which mean "computed on
   first use". This one means "stored at its own key". I'd call it `Cell`, `Slot`, or `Detached`.
7. **`remove` vs `discard`**: good names. But nothing in `GroupedMap`/`IterMap` steers a unit-valued
   map toward `discard`.
8. **`UseGetForScalars`**: an odd name, but it works as a diagnostic. Keep it.
9. **Things I reached for and couldn't find:** `at_mut`, `IterSet::key_at`/`keys_page`,
   `GroupedSet`, offset paging (`keys_range`), `page_with(map_fn)`, `insert_new`/vacant entry,
   a size estimate for NEP-145 bonds, `Lazy::new(v)` usable in a struct literal, and a public
   test harness.
10. **`Map`/`Set`/`GroupedMap` are not `Removable`.** I found this by grepping `Removable for`. It
    shapes the whole schema (every nested collection in a removable record must be
    `IterMap`/`IterSet`/`Lazy`/`Vec`), so it deserves a sentence in the module doc.

---

## 5. What was good

1. **Nested collections without prefix bookkeeping.** `Token { approvals: IterMap<AccountId, u64> }`
   inside `IterMap<TokenId, Token>`, and `Member { proposals: IterSet<_>, votes: IterSet<_> }`.
   In v1 every one of those needs a `BorshStorageKey` enum variant with a hashed per-parent
   prefix:
   ```rust
   // v1
   approvals: IterableMap::new(StorageKey::Approvals { token_hash: env::sha256_array(id.as_bytes()) }),
   ```
   In v2 it's `approvals: Default::default()`, and the path is derived. This is the single
   biggest win.
2. **Remove cascades, and they're verifiable.** `nft_burn` on a token with 3 approvals and
   `Lazy` metadata, DAO `finalize`, and DAO `remove_member` all return the store to its exact
   previous key set. I asserted that with `BTreeSet` equality of `testing::store()` keys. In v1,
   forgetting `token.approvals.clear()` before `tokens.remove()` is a classic orphan bug. Here
   you can't forget.
3. **Writes only when the encoding changed.** A second `storage_deposit` for a registered
   account writes nothing. `vote` doesn't rewrite the member or proposal records it got through
   `get_mut`. A force unregister rewrites the root only because `total_supply` changed.
4. **The per-method op-count docs are accurate.** Every receipt I asserted matched the doc
   comments exactly, once I added them up: `nft_transfer (8,8,3)`, `vote (7,8,0)`,
   `nft_burn (10,2,17)` (with `discard` on the owner group), and `ft_balance_of`/`ft_total_supply` views of `(2,0,0)`/`(1,0,0)`
   with `Lazy` metadata untouched. Both times I was wrong, the doc was right.
5. **The per-call cache makes natural code cheap.** `get` then `remove` is one read. `tally()` and
   `drop_proposal` both iterate the same votes map in `finalize` with no re-reads.
6. **Runtime misuse panics are excellent.** "collection is not bound to a path; place its
   record with `insert` or `or_default()` first (in `#[init]`, call `.bind()` on the root)",
   and "page cursor key no longer exists (removed since the last page); restart from a known
   key". Both told me what to do.
7. **`testing::Receipt` and `assert_view_clean()`** make cost assertions and the "views don't
   write" rule one-liners. I'd want them public.
8. **`GroupedMap::group` on a missing group** returns an empty handle (`len() == 0`). No
   `Option` dance in `nft_supply_for_owner` or `nft_tokens_for_owner` for unknown accounts.
9. **Inline nested records at the root** (`config: Config`) behave like a plain field:
   `self.config.quorum`. No ceremony.

---

## 6. Status

| Contract | Compiles | Tests | Notes |
|---|---|---|---|
| FT (`state_v2_ft.rs`) | yes, first attempt | 14/14 pass | Includes measured NEP-145 bond (152 B for a 53-char account) |
| NFT (`state_v2_nft.rs`) | yes, first attempt | 15/15 pass | Offset (NEP-181) and cursor paging; burn-with-approvals leaves no keys |
| DAO (`state_v2_dao.rs`) | yes, after 1.1 and 1.2 | 13/13 pass | `remove_member` / `finalize` cascades verified by key-set equality |

All three run with
`NEAR_SANDBOX_BIN_PATH=/usr/bin/true cargo test -p near-sdk --features unstable,unit-testing --test state_v2_{ft,nft,dao}`.
The errors in 1.3 through 1.7 came from temporary code (a throwaway test file, and methods added
to the DAO file and then reverted), written to see what a developer would hit with the obvious
first attempt. None of it is committed.
