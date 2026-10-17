# NEAR SDK storage v2

Oct 16, 2026 · @Ricky · Design proposal, request for comment

## Summary

This proposes `near_sdk::state`, a new opt-in storage module for NEAR contracts. The developer writes nested structs with a numeric tag on every field. The SDK derives every storage key from the struct path, writes each key once per call, and states and tests the cost of every operation. What that buys:

- **No hand-picked prefixes.** Keys are a function of the schema and the path, so two values cannot share a key. This closes the collision class behind the October 2026 hot-bridge exploit.
- **No migration for field changes.** Records are tagged, not positional. Add, remove or reorder a field and deploy.
- **Stated, bounded gas.** Every operation documents its reads, writes and bytes. Cost never depends on neighbors or on history, and CI checks the formula.
- **v1 untouched.** `store` and `collections` keep working byte for byte. A deployed contract adopts the new module one field at a time.

One protocol change is proposed, metered range iteration. Three more are listed for discussion. A prototype on branch `proto/storage-v2` compiles the example below as printed and checks the cost table; its findings are folded in.

**How to read this.** The next two sections, the code and the seven guarantees, are enough to judge the proposal. The rest is mechanism and evidence. Comment anywhere; the Open questions at the end are where comments help most.

Terms: Borsh is NEAR's binary serialization format. `collections` and `store` are the two v1 modules. A neighbor is another entry in the same collection. The mock is the in-memory host that `cargo test` runs contracts against. 1 Tgas = 1,000 Ggas = 1,000,000 Mgas.

## What the developer writes

A contract is a set of nested structs. Every field carries a tag, every collection is a field like any other, and the tags are the whole layout. There is no `StorageKey` enum, no `new(prefix)` and no `Default` impl hiding a prefix. The example is a small exchange with per-account balances and per-market order books.

```typescript
use near_sdk::state::{Map, GroupedMap, Lazy, Page};

#[near(contract_state(layout = v2))]
pub struct Exchange {
    #[key(1, required)] owner: AccountId,   // no Default, so a missing tag is an error
    #[key(2)] fee_bps: u16,
    #[key(3)] accounts: Map<AccountId, Account>,
    #[key(4)] orders: GroupedMap<MarketId, OrderId, Order>,
    #[key(5)] metadata: Lazy<Metadata>,          // large, loaded only when touched
}

#[near(storage)]
pub struct Account {
    #[key(1)] nonce: u64,
    #[key(2)] balances: Map<TokenId, u128>,      // lives at 3/<account>/2/<token>
}

#[near]
impl Exchange {
    pub fn deposit(&mut self, token: TokenId, amount: U128) {
        let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
        *acct.balances.entry(token).or_default() += amount.0;
    }

    // writes below the record without reading it: one existence check, no decode
    pub fn set_balance(&mut self, id: AccountId, token: TokenId, amount: U128) {
        self.accounts.at_mut(&id).balances.insert(token, amount.0);
    }

    // one read below the root; the Account record is never loaded
    pub fn balance_of(&self, id: AccountId, token: TokenId) -> U128 {
        self.accounts.at(&id).balances.get(&token).copied().unwrap_or(0).into()
    }

    // cursor pagination; offset pagination stays available and cheap
    pub fn balances(&self, id: AccountId, after: Option<TokenId>) -> Page<TokenId, u128> {
        self.accounts.at(&id).balances.page(after.as_ref(), 50)
    }

    pub fn cancel(&mut self, market: MarketId, order: OrderId) {
        self.orders.group_mut(&market).remove(&order);
    }
}
```

Gone: the `StorageKey` enum and `new(prefix)` calls, the migration for an added or removed field, the parent read before a nested lookup, and the hand-rolled map of maps.

**Two new attributes.** `#[key(n)]` gives a field its identity. `#[near(storage)]` marks a struct as a tagged record that may contain collections, in place of the Borsh derive v1 needs on value structs. A struct without it is stored as an opaque Borsh leaf, which is fine for plain values. A field whose type has no `Default` says `#[key(n, required)]` or is an `Option`; the derive's error says so. The existing `#[near]` on the impl also flushes writes for a v2 root, as described under One cache, one flush.

**`at` walks, `get` reads, `at_mut` writes below.** `self.accounts.at(&id)` is a generated `AccountAt<'_>` bound to the path for `id`. It reads nothing: it exposes the record's collection and `Lazy` fields as handles at the nested path, and nothing else. Scalars come from `get(&id)`, which reads and decodes the record. A view is valid for an absent key, where nested lookups return `None` and `len()` is 0, and anything read through it lives as long as the borrow of the parent collection, so `let bal = self.accounts.at(&id).balances.get(&token);` compiles. 

**Two ways to write.** `get_mut(&id)` and `entry(id).or_default()` return `&mut Account`, the decoded record with its collections bound. `at_mut(&id)` returns `AccountAtMut<'_>`, the same handles mutably, after one existence check and no decode, so a vote lands under a proposal without reading its description; it panics if the record is absent. An absent record is created when `or_default` first touches it, and an existing record is rewritten only if one of its scalars changed. `balance_of` above is one read below the root and never decodes the `Account` record. Two records at once, as in a transfer, are written as two statements; the cache makes the second lookup free (prototype: 5 reads, 2 writes for a transfer between two existing accounts):

```rust
*self.accounts.entry(from).or_default().balances.entry(t).or_default() -= x;
*self.accounts.entry(to).or_default().balances.entry(t).or_default() += x;
```

**Constructing a record.** A `#[near(storage)]` struct is built like any struct, and the derive gives it `Default` unless a field is `required`. Its collection fields are unbound until the record is placed with `insert` or `or_default()`, which binds them to the path; touching an unbound collection panics with a message. `Lazy::new(v)` is the one exception: it holds its value and writes it when the record is bound, so a struct literal can carry it. The wrapper binds the root's collections when the method returns, so an `#[init]` body that seeds a collection builds the root with `Exchange { .. }.bind()` first. A record never moves: `remove` clears what it owns and returns it with its collections detached, so read a record's collections before removing it, and move data between keys by remove and rebuild. `Page<K, V>` is `{ items: Vec<(K, V)>, next: Option<K> }` and serializes to JSON; `page_with(after, limit, |k, v| ..)` builds one from records, which cannot be cloned. `iter()` yields decoded `(K, V)` pairs at two reads per entry; `keys()` costs one.

## Seven guarantees

These are the contract between the SDK and the developer. Each is checked by a test in CI.

- **Addressable.** Every value has a key you can compute from the schema and the path, without reading state. No hand-picked prefixes, no parent read before a nested lookup, no view methods just to read state. Two paths cannot share a key, which is the collision fix.
- **Local.** An operation on one entry touches its own path plus a fixed amount of bookkeeping, never another entry's value. This removes the removal cost cliff. Two operations scale by the caller's choice: clearing a whole record, with the entries it owns, and `Vector::swap_remove`, which moves the last element.
- **No debt.** Host calls per operation depend only on the operation, and bytes moved only on the keys and values involved, never on what happened before. No free lists, no holes, no iteration that slows with age.
- **One write per key per call.** Writes land at the end of the method, once per modified key, in a fixed order, from the one root a call holds. View methods never write. Nothing happens in `Drop`. A v1 collection kept as a root field during migration keeps v1's write-on-drop, after the v2 flush.
- **Linear iteration.** A page of n entries costs two reads plus two per entry, or one per entry for keys alone, whatever the collection's size or age.
- **Permanent layout.** A deploy never moves or rewrites existing data by itself. Adding, removing or reordering a field needs no migration, as long as an added field is `Default` or an `Option`; a `required` field is for fields present since the record's first write.
- **Stated cost.** Every operation documents its formula in reads, writes, removes and bytes, and CI asserts it with a 100 KiB entry next to the one under test.

Together these make gas a function of the operations a method performs and the byte sizes it chooses, and of nothing else the contract controls. The one term outside the SDK's reach is the trie-node fee on writes and removes, about 2.3 Ggas per node since protocol version 82. Measured on the real runtime with near-contract-sim: a read touches no trie node at any size, because reads are served from flat storage. One write touches 9 nodes in an empty trie and 19 in a contract with 100,000 keys on a chain with 100,000 accounts, 20 to 43 Ggas. Each tenfold growth in keys adds about one node, and hashed placement keeps the trie balanced.

## The problem today

A NEAR contract's storage is one flat map from byte keys to byte values. The SDK's v1 collections carve it up by prefix: a `LookupMap` under `b"a"` stores entries at `a ++ key`. The developer picks the prefixes, nests collections by building longer ones, and keeps the top-level struct as one Borsh blob under the key `STATE`. Everything below follows from that.

| Pain | What the developer does today | What goes wrong |
| --- | --- | --- |
| Choosing the layout | Pick a unique prefix per collection; for a per-account map, put the account id in the prefix | Overlapping prefixes corrupt data silently. Raw keys and 32-byte hashed keys share one space, so a caller-controlled key can equal another collection's entry (shown below). |
| Predicting cost | Read the big-O in the docs and guess a gas budget | Removing from `IterableMap` rewrites a neighbor's whole value, so one removal can cost ten times another. `UnorderedMap` iteration slows after removals. Writes happen in `Drop`, and the root blob is rewritten on every mutating call. |
| Changing the schema | Write a migration and run it in a separate deploy | Adding one root field needs a migration. Changing a nested value struct means looping over every entry. A `LookupMap` cannot be iterated, so it cannot be migrated without an off-chain key list. |
| Picking a type | Choose among `store` and `collections`, seven types each, plus `Lazy` and `LazyOption` | Old tutorials point at legacy types. The differences are about internals, not about what the developer needs. |
| Reading state | Write a view method for every shape a frontend needs | Boilerplate, and views drift from the real layout, because keys cannot be computed off-chain. |

**The collision, concretely.** Hashed collections store an entry at `sha256(prefix ++ key)`, 32 bytes with no trace of the prefix. Raw collections store at `prefix ++ bytes`. A caller who controls most of a raw key's bytes, and can make it exactly 32 bytes long, can search for inputs until it equals a hashed entry they care about. In October 2026 this was confirmed as a critical exploit in a deployed bridge. No SDK invariant forbade it. Safety rested on Borsh length prefixes and account-id grammar, which held in most contracts and not in that one.

&#91;embedded content: v1 key space · the hot-bridge shape\]

## Keys are paths from the root

A value's key is its path from the root: a root byte, then one segment per hop. A field hop adds the field's tag, a collection adds a byte naming which of its internal tables is meant, and a map entry adds its key led by a hash of itself. Anyone with the schema can compute the key of any value without reading state, and two different paths cannot produce the same bytes.

Scalars live inside their record. Collections and `Lazy` fields get their own subtree, so nesting adds segments instead of a hand-picked prefix. The segments:

| Segment | Encoding | Prefix-free because |
| --- | --- | --- |
| Root | one byte, configurable per contract, default fixed in the spec | fixed length |
| Field | the `#[key(n)]` tag as a LEB128 varint, one byte for tags 1 to 127 | varints are self-delimiting |
| Kind | one byte per collection table: `00` entries, `01` index, `02` slots, `03` header | fixed length |
| Entry | `sha256(borsh(k))[..16] ++ borsh(k)`, the map key led by 16 bytes of its own hash | Borsh of one type is a prefix-free code when its decoder reads forward exactly what it needs, which holds for derived impls and the standard types. Map keys are bounded by a sealed `StorageKey` trait, implemented for the standard types and emitted by the SDK's Borsh derive for user types; a hand-written Borsh impl is not accepted |
| Slot | `u32` little-endian index, used by `Vector` and the `Map` key vector | fixed length |

The hash in front of an entry is for placement, not uniqueness; Borsh already gives uniqueness. A caller cannot choose where their key lands in the trie or line it up with another key shape, because the 16 bytes after the kind byte are a function of the key itself. Sixteen bytes matches Substrate's `Blake2_128Concat`.

The protocol rejects storage keys over 2,048 bytes, and nesting compounds: three nested maps carry three map keys in every entry's path. So every `StorageKey` type declares a maximum length, fixed for integers, 64 for `AccountId`, 512 by default for strings and byte vectors, and the derive sums the worst case along each path and refuses a schema that could exceed the limit (a `cargo build` or `cargo test` error; `cargo check` does not reach it). A key over its declared maximum panics at the call with the limit named, so an oversized caller-supplied key fails its own transaction and nothing else. `StorageKey` is derive-only, and the derive generates the key encoding itself, so a hand-written Borsh impl never shapes a key. Recursive schemas are out of scope, since the bound needs a finite schema. Within the limit, per-byte key costs fall on the caller who chose the key.

**Worked example.** For the `Exchange` contract above with root byte `R`, `alice.near` as the account and `usdt` as the token:

| Path | Key bytes | Length |
| --- | --- | --- |
| Root record | `R` | 1 |
| `accounts[alice]` record | `R 03 00 h16(alice) 0A000000 "alice.near"` | 33 |
| `accounts[alice].balances[usdt]` value | `R 03 00 h16(alice) 0A000000 "alice.near" 02 00 h16(usdt) 04000000 "usdt"` | 59 |
| same entry's index slot | `... 02 01 h16(usdt) 04000000 "usdt"` | 59 |
| `balances` key vector, slot 7 | `... 02 02 07000000` | 39 |
| `balances` header (length) | `... 02 03` | 35 |
| `orders[market].group` entries | `R 04 00 h16(market) borsh(market) 00 h16(order) borsh(order)` | varies |

&#91;embedded content: walk the path · interactive\]

`borsh("alice.near")` is the 4-byte little-endian length `0A000000` followed by the ten bytes. In the `balances` rows the first `02` is the field tag of `balances` and the next byte is the kind. At 70 Mgas per key byte on a write, the deepest key here adds about 4 Ggas to a 64 Ggas base. Key length is not where gas goes.

## Records are tagged, so field changes need no migration

A `#[near(storage)]` struct is stored as a tagged record, not as positional Borsh. The tags already exist for keys, and the record format reuses them.

```text
record  := field*
field   := varint(tag) varint(len) bytes[len]
```

&#91;embedded content: record bytes · adding a field\]

- A scalar field is `tag`, `len`, then its Borsh bytes. A nested `#[near(storage)]` struct is a nested record, so it evolves too.
- A collection or `Lazy` field writes nothing into the record. Its state lives under child keys. An `Account` with a nonce and a balance map is a 10-byte record.
- A tag missing from the record reads as the field's `Default`, or fails to decode if the field is `required`. A tag the code does not know is preserved, as below. Tags are strictly ascending, tag 0 and non-canonical varints are rejected, so every record has exactly one encoding.
- Any other type, including a plain Borsh struct, is an opaque leaf. This is how v1 collections can sit inside a v2 record.

**What this buys.** Adding a field is a deploy; every existing record reads it as its default. Removing a field is a deploy; its bytes drop the next time each record is rewritten. Reordering is free, since the tag is the identity. Changing a field's type still needs a migration. A scalar whose Borsh length changes fails loudly at decode. A swap the record cannot see is silent: a scalar of the same length, a reordered enum inside a leaf, or one collection type for another under the same tag. The ABI records every field's type and collection kind, and `storage diff` compares two ABIs, so this is caught at build time.

**Cost.** Two bytes per field for tags and lengths under 128, under 1 Ggas per write for a typical struct. The ABI gains a record schema so clients and `cargo near` can decode records without the contract.

**Why it is in the first release.** A record encoding cannot be switched later without rewriting every stored record, which is the migration this design exists to remove. It freezes alongside the key format.

**Unknown tags are preserved.** Code that meets a tag it does not know keeps those bytes and writes them back through `get_mut` and `entry`, so rolling back to an older version cannot destroy data the newer one wrote. A blind `insert` replaces the record whole and drops them. The cost is one boxed slice per record, empty in the common case. A deliberately removed field is listed as `#[near(storage, reserved(3))]`; reserved tags are dropped on rewrite and cannot be reused for a different type.

## Collections and cost bounds

One family, each type with a stated bound. Operations are counted, not estimated: a read is about 56 Ggas plus bytes, a write about 64 Ggas plus bytes and trie nodes, a remove about 53 Ggas plus bytes and nodes. An overwrite also pays 32 Mgas per byte of the value it replaces. Every bound below depends only on the caller's own key and value, and for a `Map` removal on the key that moves into the gap, never on a neighbor's value.

Which type to reach for:

- `Map<K, V>`: the default. A lookup table plus a key vector, so it has `len`, `page`, iteration, and a record holding one can be removed as a whole. Two small reads and four writes on a new key. `insert` overwrites and reports whether the key was new, so `require!(votes.insert(who, v), "already voted")` is one call; `entry(k).or_insert(v)` is first-write-wins.
- `LookupMap<K, V>`: lookup only, one write per insert. Entries can be removed one by one but never iterated, so a record holding one cannot be removed as a whole (see Removing a record). Choose it knowingly: the choice is permanent.
- `Vector<T>`: dense slots with push, pop, swap-remove and indexing.
- `Set<K>` and `LookupSet<K>`: the two maps with a unit value. A `GroupedMap<G, K, ()>` is a grouped set at no extra cost.
- `Lazy<T>`: one value at its own key, read only when touched. For large or rarely used fields.
- `GroupedMap<G, K, V>`: one `Map` per group under a single field, so order books per market need no wrapper struct. `group(&g)` returns a view that pages and counts within the group. Groups are not iterated unless the contract opts in.
- `OrderedMap<K, V>`: sorted iteration and range queries, after the first release.

| Type | Keys it owns | get | insert new | update existing | remove | Removal cost scales with |
| --- | --- | --- | --- | --- | --- | --- |
| `LookupMap<K, V>` | `00` entries | 1 read; `contains_key` and `at_mut` are 1 host call and no bytes | 1 write, no existence read. A value that owns collections adds 1 existence check and panics if present | 1 write plus the old value's bytes; `replace`, which returns the old value, adds 1 read | 1 read, 1 remove; `discard` skips the read, and so does a unit value; plus clearing any collections inside the value | removed value size, plus one remove per key the value's own collections hold |
| `Map<K, V>` | `00` entries, `01` index, `02` key slots, `03` header | 1 read | 1 index read, 1 header read, 4 writes: entry, index, key slot, header | 1 write after a read in the same call; `insert` alone is 1 index read and 1 write | 1 entry read (skipped by `discard`), 1 index read, 1 header read, 2 removes for entry and index, 1 read and 1 remove of the last slot, 2 writes for the moved key and its index, 1 header write. Removing the last key skips the moved-key writes; emptying the map removes the header | removed value size, own key size, and the size of the last key, which moves into the gap |
| `Vector<T>` | `00` slots, `03` header | 1 read | 1 header read, 1 write, 1 header write | 1 write | pop: 1 header read, 1 slot read for the returned value, 1 remove, 1 header write; swap\_remove: plus 1 read and 1 write of the last slot | last element size |
| `Set<K>`, `LookupSet<K>` | as `Map` and `LookupMap` with a unit value | same | same | same | same, minus the value read | key size |
| `Lazy<T>` | its own path | 1 read | 1 write | 1 write | 1 remove | value size |
| `GroupedMap<G, K, V>` | a `Map` per group under `00 ++ entry(g)` | 1 read | as `Map` | 1 write | as `Map`, plus 1 header remove when the group empties | as `Map` |
| `OrderedMap<K, V>` | `00` entries, `01` tree nodes, `03` header | 1 read | 1 write plus O(log n) small node writes | 1 write | 1 remove plus O(log n) small node writes | tree depth and key size |

**Removing a record.** A value's collections live under child keys, so removing the record alone would orphan them. `remove` therefore requires `V: Removable`: every plain Borsh value is, and a `#[near(storage)]` struct is when its collections can all be iterated (`Map`, `Set`, `Vector`, `Lazy`, and `GroupedMap` with group iteration on). The derive clears each collection before the record goes, one header read, then one slot read and three removes per `Map` entry. A struct holding a `LookupMap`, a `LookupSet` or a default `GroupedMap` is not `Removable`, and `remove` on it does not compile, with the error naming the field and the fix: use `Map`, or keep the record and clear fields by hand. A v1 collection is a plain Borsh value to the derive, so it belongs at the root, where `remove` never reaches it. Clearing is the one operation whose cost scales with what the record owns, so `clear(n)` removes at most n entries per call, and the `bounded_work` lint flags an unbounded `remove` on a record with collections. The same rules govern the old entry in `Migrating`. Protocol ask A turns the clearing into one host call.

**Two small keys instead of one big rewrite.** v1 `IterableMap` keeps each entry's key-vector index inside its value, so removing a non-last entry rewrites the moved key's whole value. v2 gives the index its own 4-byte slot: one extra small write on insert, and removal bounded by the caller's own value plus the moved key, never by a neighbor's value. The length gets its own 4-byte header key for the same reason: an insert never rewrites the parent record, and `at(&id).balances.len()` is one read. The last key moves into the gap, so iteration order is not insertion order after a removal.

&#91;embedded content: removal · v1 IterableMap vs v2 IterMap\]

**Pagination.** `page(after, limit)` reads the cursor key's index slot and the header, then walks the key vector: two reads, plus one key slot and one entry per item. If the cursor key was removed since the last page, the call panics with a message saying so and the client restarts from a known key. A removal elsewhere between pages moves the last key into the gap, so a key can be skipped; cursor pages are exact only across calls with no interleaved removals, and the type's docs say so. `iter().skip(from).take(limit)` costs nothing per skipped element, since the key vector is indexed, and is the shape for NEP-181 offset enumeration and for public views that page while other calls remove. Each map hop also costs one sha256 of the key, about 4.5 Ggas against 56 for a read.

**Removal returns the value.** The host remove runs at flush, so `remove` reads the value it returns now, one read on top of the remove. The host charges 11.5 Mgas per byte of the removed value either way. `discard` skips the read and the decode.

`UnorderedMap` and `UnorderedSet`, with their free-list holes, do not come to v2, and `IterableMap`'s name does not either: the iterable type is the default here, so it takes the plain name. `OrderedMap` ships after the first release on the same key scheme; its tree compares decoded keys, so it needs no order-preserving encoding.

## The root is a record too

The root struct is one tagged record at the root key, read and decoded once at the start of every call and written once at the end of a mutating call, only if a scalar changed. That read is the one cost every method pays, so root scalars stay small and anything large goes in `Lazy`. Collections and `Lazy` fields are references out of it and cost nothing until touched. One rule covers every level: a struct is a record, a collection is a subtree. A call holds one root; the wrapper creates it, and the reference-returning `get` relies on that.

**What happens to `STATE`.** A v2 root lives at the root byte, not at `STATE`. The `#[near]` wrapper reads it before the call and flushes after, through a pair of load and store hooks on `ContractState` that carry no Borsh or `Default` bound, so a v2 root needs no `PanicOnDefault`. `Drop` is a no-op in release builds and, in tests, a debug assertion that the wrapper flushed, skipped while a panic is unwinding.

**Migrations that remain.** Changing a field's type, splitting a map, or moving data between paths still needs code. A `fn migrate(old: RawRecord) -> Self` hook receives the old root as raw tagged fields, so renaming a tag or converting one field is a few lines. Versioned roots via an enum stay possible but are no longer the default advice.

## One cache, one flush

In v1 every collection has its own cache and flushes in `Drop`, in whatever order the fields drop, so a method's write pattern depends on code structure. In v2 there is one cache per call, keyed by storage key bytes. A read fills it, a write marks the key dirty, a remove marks it deleted. Slots are boxed and never freed within a call, so `get` can return a reference into the cache, and a loop over ten thousand entries holds ten thousand decoded values in memory, which is what the `bounded_work` lint is for.

The `#[near]` wrapper flushes at the end of the method: dirty keys once each in key order, then the root record last, so it reflects everything else, then it drops the cache, so a v1 collection held as a root field writes in its own `Drop` after the v2 keys. A balance updated ten times is one write. An insert then a remove of a key the call has already seen to be absent costs nothing. A view method cannot write: its handles come from `&self`, so the compiler stops it.

&#91;embedded content: one call through the cache · interactive\]

The same cache gives exact storage accounting. Every write and remove passes through one place, so after the flush the SDK knows the call's byte delta, including the 40 bytes the runtime charges per record, and exposes it as `env::storage_delta()` to a post-flush hook and to the storage-accounting guard. It cannot be known earlier, because the old length of an overwritten key only comes back from the host. Inside the method body `storage_usage()` is pre-flush, and the mock warns when a `state` contract reads it there, since the NEP-145 before-and-after pattern no longer measures anything.

It is also less code: one cache replaces seven, and a key builder of LEB128 and sha256 replaces a Borsh serializer per prefix enum. Contract loading gas is proportional to wasm size, so the saving lands on every call.

## Why the guarantees hold

Each guarantee rests on one mechanism, makes a claim no wider than the check behind it, and has a harness that fails if the mechanism breaks.

| Guarantee | Mechanism | Claim | Harness and bounds |
| --- | --- | --- | --- |
| 1 Addressable | Keys are a pure function of schema and path; only a `Path` can make a `Key`, there is no `Key::from_bytes` | Type-enforced; bounded model-checked | Kani over two symbolic paths, depth ≤ 4, tags below 2^14, map keys ≤ 8 bytes, with `h16` as free symbolic bytes so the claim holds even if the hash collides: equal bytes imply equal paths, and a prefix implies an ancestor. Also run as a property test and a fuzz target with the real sha256 |
| 2 Local | No collection stores anything inside another entry's value; `Map` keeps its index in its own slot; `Vector::swap_remove` is the stated exception | Bounded model-checked; property-tested | A recording host and a symbolic operation on a model collection of ≤ 3 entries: the touched keys stay inside the operation's own path plus header, index and moved slot. Property test: a removal beside a 100 KiB neighbor costs the same as beside a 1-byte one |
| 3 No debt | No free lists or tombstones; swap-remove keeps slots dense | Differentially tested | Host-call counts on a fresh collection equal the counts after ≤ 6 symbolic prior operations, and after 10,000 random ones in a property test |
| 4 One write per key per call | One per-call cache flushed by the wrapper; collections hold a cache handle and cannot reach the host; one root per call | Compiler-enforced; bounded model-checked | The host write is private to the cache module; a `disallowed_methods` lint covers the rest of the crate. Kani over ≤ 6 symbolic cache operations then a flush: each key written at most once, in key order with the root last, and a view flush writes nothing |
| 5 Linear iteration | Dense `u32` key slots | Property-tested | A page with a cursor costs 2n + 2 reads for key-value pages and n + 2 for keys, for n ≤ 64 over random maps of any size and history |
| 6 Permanent layout | Tags instead of positions; missing tags default, unknown tags are preserved | Bounded model-checked; conformance-tested | Kani over records of ≤ 4 fields of ≤ 16 bytes: decode after encode is the identity, and a record written by one schema and read by another round-trips with defaults and preserved bytes. Golden byte vectors are frozen with the format spec |
| 7 Stated cost | Every operation's formula in rustdoc | Unit-tested; simulation-tested | One test per operation per type compares the formula to the mock's counting receipt against a 100 KiB neighbor; the same scenarios run in near-contract-sim on the real runtime |

Each property is one bolero harness: the random engine in `cargo test` on every PR, libFuzzer and Kani in a nightly job with the bound reported next to the claim. None is a proof in the strict sense; they are bounded checks with stated bounds. The one unbounded argument, that concatenating prefix-free codes gives a prefix-free code, is a half-page theorem in the format spec. `cargo-mutants` runs over the encoder and key builder so a green suite is known to check something.

Guarantee 1 carries security weight, so here is the argument that two different paths never produce the same key bytes:

- Keys are paths, and paths are unique. Each segment kind is prefix-free among its siblings, and the schema fixes which kind comes next, so two keys are equal only if every segment is equal.
- Nothing the caller controls can be placed. A map key enters a storage key only as `h16(k) ++ borsh(k)`. The caller chooses k, but the 16 bytes in front are a function of k. Tags, kind bytes and slot indices are never caller-controlled.

**Hot-bridge replayed.** The exploit needed a 32-byte raw key `06 | chain | len | receiver` whose first 6 bytes were fixed and whose 26 tail bytes the caller chose. Here the pending-withdrawals entry is `R 06 00 h16(chain, receiver) borsh(chain, receiver)` and the balance entry is `R 03 00 h16(acct) borsh(acct) 02 00 h16(token) borsh(token)`. They differ at the second byte, the field tag, before any caller byte appears. The common v1 nested-prefix shape, an account id baked into a prefix, gets the same treatment: the id appears only as `h16(acct) ++ borsh(acct)` after a fixed tag, so it cannot aim at anything.

**What the format does not protect against.** Direct `env::storage_write` under the root byte, which the simulator flags. A v1 collection whose raw prefix starts with the root byte, covered under Coexistence. And it does not make a single storage operation cheaper; it removes the paths that touched other entries.

## Coexistence and migration from v1

Nothing in `store` or `collections` changes, and no default hasher moves. Moving deployed data silently would be worse than the bug.

**New contracts** use `near_sdk::state` only and never see a prefix.

**Deployed contracts** adopt it in three steps, each optional and each a normal upgrade.

- **Move the root.** Switch to the v2 layout and write a one-time `migrate` that reads the old `STATE` blob into the new root record. Existing v1 collections stay as fields of the new root, with their old keys, their own cache and their write-on-drop, which runs after the v2 flush. They belong at the root: nested inside a record they still write, but `remove` on that record does not clear them, and the derive can only reject them when the field's type path names `store` or `collections`, since v2 reuses the `LookupMap` name. The big maps never move; one record is rewritten.
- **Add new fields as v2 types.** Every field added from now on is path-derived and collision-free. Most contracts stop here.
- **Rekey an old collection when it is worth it.** `Migrating<V1, V2>` wraps an old and a new map: reads try the new path and fall back to the old, writes go to the new path and delete the old entry. `IterableMap` and `Vector` know their keys, so `migrate_batch(n)` moves entries across calls and the wrapper comes off when the old side is empty. A v1 `LookupMap` cannot be iterated, so for it the lazy path is permanent unless an indexer supplies the keys: every miss costs two reads, and the old digest keys remain a collision target as long as any raw-key writer remains.

**The root byte.** All v2 keys start with one byte. A v1 prefix starting with that byte, or an empty v1 prefix, could alias a v2 key. Three guards: the default is outside the range small `BorshStorageKey` enums produce, it is configurable per contract, and the simulator scans real state and asserts that no v1 key starts with it. `STATE` cannot collide; its first byte is `S`.

**Hardening v1 meanwhile.** Independent of v2, a `Namespaced` hasher producing `prefix ++ sha256(key)` fixes the collision shape on new fields with a one-type-parameter change, since every hashed key then carries its enum tag. It is a sixty-line, non-breaking addition to a sealed trait, and the v2 scheme in miniature.

## Tooling

The layout is machine-readable, so most safety checks become build artifacts rather than review items. In the order they ship:

**Storage schema in the ABI, from the first release.** The derive that builds keys also emits a `storage` section into the ABI the contract already embeds: every path from the root, its tag, collection kind, key and value schemas, the record format version and the root byte. It costs nothing at runtime, and everything below reads it.

**Checks in near-contract-sim.** The simulator runs the contract on the real runtime, so it sees real keys and real gas. Given the schema, it asserts that every write lands under a declared path and that no v1 prefix starts with the root byte or is a prefix of another live v1 prefix. For v1 code it flags the collision shape directly: a raw-key collection writing 32-byte keys in a contract that also writes digests. That check would have failed hot-bridge's own tests. The unit-test mock keeps only a counting receipt, so `cargo test` stays fast.

**Gas receipt and gas-bound tests.** The mock counts reads, writes, removes and bytes per key per call; the simulator reports the same receipt in real Tgas. `#[gas_bound(reads = 1, writes = 2)]` asserts the counts in a unit test, and the SDK's CI runs one per operation per type, so the cost table above is checked rather than promised. A `bounded_work` lint, opt-in at first, warns on `iter()` without `take` or `page` inside a `#[near]` method.

**cargo-near commands, next version.** `storage check` validates the schema and the v1 prefix set for overlap, `storage decode` prints state as paths and typed values from a dump or RPC, and `storage diff` lists paths whose keys or types changed between two ABIs, the input to a migration. There is no lockfile; the deployed wasm embeds its ABI.

## Protocol asks

Everything above runs on today's host functions. The protocol cannot make moving bytes cheaper, but it can let contracts avoid moving them. A is proposed; B to D are for discussion. Runtime-team review of this section is requested before the proposal goes beyond NEAR One.

**A. Metered range iteration, as a NEP.** The trie is sorted by key, and `storage_iter_prefix`, `storage_iter_range` and `storage_iter_next` still exist in nearcore as stubs returning `Deprecated`. A revived version takes a start cursor, an end bound and a page limit, and charges per trie node visited plus per byte returned, with a hard page cap to bound proof size. This key format is built for it: every map's entries sit contiguously under `ns ++ 00`, so `Map` collapses to `LookupMap` plus a header, removal is one `storage_remove`, and cursors are plain keys.

**B. Conditional writes.** `storage_write_if(key, expected_hash, value)` writes only if the current value's hash matches. Every trie leaf already holds `ValueRef { length, hash }`, so the check needs no value read. This makes callback guards cheap for large values.

**C. Content-addressed values.** Leaves already reference values by hash, and values over 4,000 bytes are kept out of the inlined flat-storage entry. Exposing that gives deduplication and a `storage_move` charged per key byte rather than per value byte. Useful for large metadata and migrations; not needed for the collections above.

**D. Storage payment in the runtime.** Minimal version: apply a call's attached deposit to its storage delta and refund the rest, so users cannot drain a contract by filling its storage. Full version: a payer per key, refunded on delete, which adds per-key state and needs its own cost model.

Storage accounting and callback guards follow in a later phase as explicit APIs: a guard object created at the top of a method and settled on return.

## Rollout and first PRs

`state` ships behind the `unstable` feature beside `store` and `collections`, and v1 keeps working throughout.

- **Harden v1 now.** Two small additive PRs: the `Namespaced` hasher, and a mock-time check that flags a raw-key collection writing 32-byte keys in a contract that also writes digests. The hasher goes first because it fixes the collision shape on new fields today.
- **Freeze the formats.** A two-page spec: the root byte, the four segment kinds, the entry hash width, the record encoding, the format version, the key length rule and the `Removable` semantics. Reviewed before any `state` code merges, because none of it can change once data is on chain.
- **Encoding module and schema derive.** The key builder, the record encoder, the `StorageKey` and `Removable` traits, the `#[key(n)]` and `#[near(storage)]` derives with their view types, and the ABI storage section. No collections yet. The harnesses for guarantees 1 and 6 ship here.
- **Collections.** `Map`, `LookupMap`, `Vector`, `Lazy`, `Set`, `LookupSet`, the `at` accessors and `GroupedMap`, each with its gas-bound test in CI, and a public `state::testing` harness with the counting receipt, since every contract's tests want it.
- **Root integration and migration.** The v2 `contract_state` layout, the wrapper's flush, the `migrate` hook, v1 collections as root fields, `Migrating`, and a minimal `storage diff` in `near-abi`, since it is the only check for the silent type swaps noted under records. The load and store hooks move a Borsh bound off `ContractState`'s provided methods, which is visible to a downstream override that restated it; the SDK has none, and the PR says so.
- **Tooling.** The near-contract-sim checks, then the three `cargo near storage` commands.
- **`OrderedMap`**, on the same key scheme.
- **Range-iteration NEP**, written in parallel with steps 3 to 6 and taken to the runtime team.

Steps 1 to 5 are the first release. Storage accounting, callback guards and the NEP-145 derive follow once the collections have shipped.

**What the prototype showed.** Branch `proto/storage-v2` holds every collection above, the views, the three macros, the `Exchange` contract, and three contracts written against the rustdoc alone: a NEP-141 token with storage deposits, an NFT with enumeration and approvals, and a DAO with per-proposal votes. All compile and pass; the `Exchange` above compiles natively and for wasm32 as printed. A counting host confirmed the key lengths, the cost table as corrected here, neighbor-independent removal, the flush order and the pagination formula. The usability pass set the names, `at_mut`, `page_with` and `Lazy::new`. The prototype is behind `unstable`, has no Kani or fuzz harnesses, and counts host calls rather than measuring gas. `PROTOTYPE_NOTES.md` and `USABILITY_NOTES.md` on the branch list every finding.

## Open questions

Each row states the proposed answer and the evidence. Disagreement with an answer, or evidence a row is missing, is the most useful comment this document can get.

| Question | Why it matters | Proposed answer |
| --- | --- | --- |
| Entry hash width, 16 or 8 bytes | 8 saves 8 bytes per key on every operation; 16 matches Substrate and leaves more margin against trie shaping | Sixteen. Forcing k matching leading bits costs 2^k hashes, so 8 would do, but the extra 8 bytes cost under 0.6 Ggas per write. Revisit only with a profile. |
| Unknown tags on rewrite, drop or preserve | Preserving keeps data written by a newer version across a rollback, at the cost of carrying unknown bytes in memory | Decided: preserve through `get_mut` and `entry`, drop on a blind `insert`, with a `reserved` list for removed tags. |
| How the macro selects v2 | Presence of `#[key]` is implicit; a flag is one more thing to type | Explicit `contract_state(layout = v2)`. The layout is permanent and should be visible at the declaration. |
| `Map` insert at four writes | One more small write than v1 per new key, three more than `LookupMap`, in exchange for bounded removal | Accept. The index and header writes together cost about 0.2 Tgas, trie nodes included, and v1's insert rewrote the `STATE` blob instead. `LookupMap` exists for lookup-only fields. |
| Default root byte | Must avoid small enum discriminants and ASCII prefixes in deployed v1 contracts | 0xF0, configurable per contract. Every v1 prefix in the SDK's examples and standards is a small `BorshStorageKey` variant or a short ASCII string, none starting above 0x7F. The simulator asserts no overlap. |
| Group iteration in `GroupedMap` | Listing which groups exist needs a group key vector maintained on first insert and last remove | Off by default; a variant with it for contracts that need it. |
| `Lazy<T>` in the first release | Needed for large root fields once the root is a single record | Yes. It is the smallest type in the set. |
| Key length limits | The protocol rejects keys over 2,048 bytes, and a nested path can get there with two long map keys | A declared maximum per key type, summed along each path by the derive, which refuses a schema that could exceed 2,048 bytes. Strings and byte vectors default to 512 bytes. The mock warns on any key over 256 bytes, since a 1 KiB key adds 70 Ggas to every write. Part of the step-2 spec, because a cap cannot be lowered once data is on chain. |
| Writes to `env::storage_write` under the root byte | Bypasses every guarantee above | The simulator flags it. The SDK documents a `clippy.toml` snippet that denies `env::storage_write` and `env::storage_remove` in crates using `state`; clippy reads it from the contract's workspace, so the SDK cannot ship it. |
| Name of the module | `state` reads well beside `store` but is close to `contract_state` | `state`. The module already exists and holds the `ContractState` trait every `#[near]` wrapper calls, so the collections join it. `storage` would clash with `env::storage_*`; `store2` says nothing. Confirm at step 2. |
| Names | The usability pass hit `Vec` shadowing `std::vec::Vec`, and `Map` as the lookup-only type steering new schemas into a permanent no-iteration choice | Decided: `Map` and `Set` are the iterable types, `LookupMap` and `LookupSet` the lookup-only ones as in v1, `Vector` as in v1. |
| `required` or `Option` for fields without `Default` | `required` keeps the field's type; `Option` makes the absence visible in code | `required`, with the evolution rule stated under Permanent layout. |
