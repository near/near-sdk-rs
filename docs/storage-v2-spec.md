# NEAR SDK storage v2

Oct 16, 2026 · @Ricky · Design proposal, request for comment

## Summary

This proposes `near_sdk::state`, a new opt-in storage module for NEAR contracts. The developer writes nested structs with a numeric tag on every field. The SDK derives every storage key from the struct path, writes each key once per call, and states and tests the cost of every operation. What that buys:

- **No hand-picked prefixes.** Keys are a function of the schema and the path, so two values cannot share a key. This closes the collision class behind the October 2026 hot-bridge exploit.
- **No migration for field changes.** Records are tagged, not positional. Add, remove or reorder a field and deploy.
- **Stated, bounded gas.** Every operation documents its reads, writes and bytes. Cost never depends on neighbors or on history, and CI checks the formula.
- **v1 untouched.** `store` and `collections` keep working byte for byte. A deployed contract adopts the new module one field at a time.

One protocol change is proposed, metered range iteration. Four more are listed for discussion. A prototype on branch `proto/storage-v2` compiles the example below as printed and checks the cost table; its findings are folded in. The migration and gas rounds ran on nearcore's real runtime through near-contract-sim.

**How to read this.** The next two sections, the code and the seven guarantees, are enough to judge the proposal. The rest is mechanism and evidence. Comment anywhere; the Open questions at the end are where comments help most.

Terms: Borsh is NEAR's binary serialization format. `collections` and `store` are the two v1 modules. A neighbor is another entry in the same collection. The mock is the in-memory host that `cargo test` runs contracts against. 1 Tgas = 1,000 Ggas = 1,000,000 Mgas.

## What the developer writes

A contract is a set of nested structs. Every field carries a tag, every collection is a field like any other, and the tags are the whole layout. There is no `StorageKey` enum, no `new(prefix)` and no `Default` impl hiding a prefix. The example is a small exchange with per-account balances and per-market order books.

```typescript
use near_sdk::state::{CursorGone, Lazy, Map, Page};

#[near(contract_state(layout = v2))]
pub struct Exchange {
    #[key(1, required)] owner: AccountId,   // no Default, so a missing tag is an error
    #[key(2)] fee_bps: u16,
    #[key(3)] accounts: Map<AccountId, Account>,
    #[key(4)] markets: Map<MarketId, Market>,
    #[key(5)] metadata: Lazy<Metadata>,          // large, loaded only when touched
}

#[near(storage)]
pub struct Account {
    #[key(1)] nonce: u64,
    #[key(2)] balances: Map<TokenId, u128>,      // lives at 3/<account>/2/<token>
}

#[near(storage)]
pub struct Market {
    #[key(1)] orders: Map<OrderId, Order>,       // lives at 4/<market>/1/<order>
    #[key(2)] tick_size: u128,
    #[key(3)] next_order_id: OrderId,
    #[key(4, default = true)] open: bool,        // added later; older records read as open
}

#[near]
impl Exchange {
    pub fn deposit(&mut self, token: TokenId, amount: U128) {
        let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
        *acct.balances.entry(token).or_default() += amount.0;
    }

    // writes below the record without reading it: one existence check, no decode
    pub fn set_balance(&mut self, id: AccountId, token: TokenId, amount: U128) {
        self.accounts.at_mut(&id).expect("no such account").balances.insert(token, amount.0);
    }

    // one read below the root; the Account record is never loaded
    pub fn balance_of(&self, id: AccountId, token: TokenId) -> U128 {
        self.accounts.at(&id).balances.get(&token).copied().unwrap_or(0).into()
    }

    // cursor pagination; a cursor removed since the last page is `Err(CursorGone)`
    #[handle_result]
    pub fn balances(
        &self,
        id: AccountId,
        after: Option<TokenId>,
    ) -> Result<Page<TokenId, u128>, CursorGone> {
        self.accounts.at(&id).balances.page(after.as_ref(), 50)
    }

    pub fn cancel(&mut self, market: MarketId, order: OrderId) {
        self.markets.at_mut(&market).expect("no such market").orders.remove(&order);
    }
}
```

Gone: the `StorageKey` enum and `new(prefix)` calls, the migration for an added or removed field, the parent read before a nested lookup, and the hand-rolled map of maps.

**Two new attributes.** `#[key(n)]` gives a field its identity. `#[near(storage)]` marks a struct as a tagged record that may contain collections, in place of the Borsh derive v1 needs on value structs. A plain Borsh struct that should be stored whole, as an opaque leaf, opts in with `#[derive(Leaf)]`; there is no blanket for Borsh types, so every field type is a value on purpose. A v1 collection is not one: it may appear only as a root field marked `#[key(n, legacy)]`. A field whose type has no `Default` says `#[key(n, default = expr)]`, as in serde, `#[key(n, default_fn = path)]`, or `#[key(n, required)]`; `expr` must be a const expression, evaluated at compile time, so `env::*` calls and other non-const code are a compile error and every old record decodes to the same value whoever reads it; the derive's error says so. A type with no const constructor (`AccountId`, `Vec`) uses `default_fn = path`, where `path` is a `fn() -> T`. **It is called on every decode where the tag is absent, until some other change rewrites the record, so it must be pure**: a function that reads `env::*` would give the same old record a different value on every read. `Option` is never forced. `required` is allowed only for fields present since the record's first write: `storage diff`, run by `cargo near build` against the deployed wasm's ABI (not the previous local build), hard-fails a newly added `required` tag. The existing `#[near]` on the impl also flushes writes for a v2 root, as described under One cache, one flush.

**`at` walks, `get` reads, `at_mut` writes below.** The rule behind the names: `at*` is path arithmetic plus at most one existence check and never decodes; `get*` reads and decodes. `self.accounts.at(&id)` is a generated `AccountAt<'_>` bound to the path for `id`. It reads nothing: it exposes the record's collection and `Lazy` fields as handles at the nested path, and nothing else. Scalars come from `get(&id)`, which reads and decodes the record. A view is valid for an absent key, where nested lookups return `None` and `len()` is 0, and anything read through it lives as long as the borrow of the parent collection, so `let bal = self.accounts.at(&id).balances.get(&token);` compiles. 

**Two ways to write.** `get_mut(&id)` and `entry(id).or_default()` return `&mut Account`, the decoded record with its collections bound. `at_mut(&id)` returns `Option<AccountAtMut<'_>>`, the same handles mutably, after one existence check, with no read and no decode (a record being cleared is moved off its key, so the same check rules out a clear in progress; see Collections and cost bounds); it is `None` if the record is absent, following std: named accessors return `Option`, and only indexing panics. `insert` always overwrites; `Map::insert` returns whether the key was new, and, like `HashMap::insert` and `HashSet::insert`, that bool is not `#[must_use]`, so a deliberate overwrite is a plain statement (the `set_balance` line above). `try_insert(k, v) -> Result<&mut V, Occupied>` writes only if the key is absent, for every value type, and returns the bound value; `Occupied` carries nothing, because the stored value is not read. For a value type that owns collections plain `insert` does not exist (a compile error): such a record enters through `try_insert` or `entry(k).or_default()`. First-write-wins is `try_insert` or `entry(k).or_insert(v)`, and a guard is `require!(m.try_insert(k, v).is_ok(), "..")` or `m.try_insert(k, v).expect("..")`, never `require!(m.insert(k, v), "..")`, which has already overwritten when it fails. An absent record is created when `or_default` first touches it, and an existing record is rewritten only if one of its scalars changed. `balance_of` above is one read below the root and never decodes the `Account` record. Two records at once, as in a transfer, are written as two statements; the cache makes the second lookup free (prototype: 5 reads, 2 writes for a transfer between two existing accounts):

```rust
*self.accounts.entry(from).or_default().balances.entry(t).or_default() -= x;
*self.accounts.entry(to).or_default().balances.entry(t).or_default() += x;
```

**Constructing a record.** A `#[near(storage)]` struct is built like any struct, and the derive gives it `Default` unless a field is `required`. Its collection fields are unbound until the record is placed with `try_insert` or `or_default()`, which binds them to the path; touching an unbound collection panics with a message. `Lazy::new(v)` is the one exception: it holds its value and writes it when the record is bound, so a struct literal can carry it. The wrapper binds the root's collections when the method returns, so an `#[init]` body that seeds a collection builds the root with `Exchange { .. }.bind()` first. A record never moves: `remove` clears what it owns and returns it with its collections detached, so read a record's collections before removing it, and move data between keys by remove and rebuild. `Page<K, V>` is `{ items: Vec<(K, V)>, next: Option<K>, revision: u64 }` and serializes to JSON (`KeyPage<K>` likewise carries `revision`); `page_with(after, limit, |k, v| ..)` builds one from records, which cannot be cloned; like `page`, it returns `Result<Page, CursorGone>`. `iter()` yields decoded `(K, V)` pairs at two reads per entry; `keys()` costs one.

## Seven guarantees

These are the contract between the SDK and the developer. Each is checked by a test in CI.

- **Addressable.** Every value has a key you can compute from the schema and the path, without reading state. No hand-picked prefixes, no parent read before a nested lookup, no view methods just to read state. Two paths cannot share a key, which is the collision fix. The stated exception is a v1 field kept at the root whose prefix is empty or starts with the root byte; `migrate` refuses it (Coexistence).
- **Local.** An operation on one entry touches its own path plus a fixed amount of bookkeeping, never another entry's value. This removes the removal cost cliff. Two operations scale by the caller's choice: clearing a whole record, with the entries it owns, and `Vector::swap_remove`, which moves the last element.
- **No debt.** Host calls per operation depend only on the operation, and bytes moved only on the keys and values involved, never on what happened before. No free lists, no holes, no iteration that slows with age.
- **One write per key per call.** Writes land at the end of the method, once per modified key, in a fixed order, from the one root a call holds. View methods never write. Nothing happens in `Drop`. A v1 `store` collection kept as a root field during migration keeps its write-on-drop, after the v2 flush; a v1 `collections` type writes through in the method body, before it.
- **Linear iteration.** A page of n entries costs two reads plus two per entry, or one per entry for keys alone, whatever the collection's size or age.
- **Permanent layout.** A deploy never moves or rewrites existing data by itself. Adding, removing or reordering a field needs no migration, as long as an added field has a `Default`, a `default = expr` or a `default_fn = path`; a `required` tag is only for fields present since the record's first write, and `cargo near build` fails on a newly added one. A removed field's bytes persist until its tag is listed in `reserved(n)`.
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
| Flag | one byte `00` after a map entry whose value owns collections: the record's clearing flag key, present only while `remove_step` is clearing it across calls | `00` is tag 0 as a varint, which no field may use, and no other tag's varint starts with `00`; it is always the last segment |

The flag key is the record's key plus one byte, so it sits beside the record in the trie and sorts before every key below it. It cannot be a field path under the record, since tags start at 1, and it cannot be a sibling entry, since no `borsh(k')` equals `borsh(k) ++ 00` and the hash in front differs anyway. It is longer than the record's key by one byte and shorter than any key below the record, so every schema the 2,048-byte bound accepts also fits its flag keys. For `markets[7]` it is `R 04 00 h16(7) 07000000 00`, 24 bytes.

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
| `markets[market].orders[order]` value | `R 04 00 h16(market) borsh(market) 01 00 h16(order) borsh(order)` | varies |

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
- An opaque leaf is stored as its Borsh bytes: the integers, `bool`, `String`, `AccountId`, `Vec`, `Option`, arrays and tuples of leaves, a few `near_sdk` types, and any type with `#[derive(Leaf)]`. Nothing else is a field type. v1 collections are not leaves, so one inside a record is a compile error naming the field ("`store::LookupMap<u32, u32>` is not a storage v2 value"), including when `use near_sdk::store::LookupMap;` shadows the `state` glob import; at the root they are `#[key(n, legacy)]` fields, stored as their Borsh bytes (the prefix), exactly as before.

**What this buys.** Adding a field is a deploy; every existing record reads it as its default. Removing a field is a deploy; its bytes stay in each record, preserved like any unknown tag, until the tag is listed in `reserved(n)`. Reordering is free, since the tag is the identity. Changing a field's type still needs a migration. A scalar whose Borsh length changes fails loudly at decode. A swap the record cannot see is silent: a scalar of the same length, a reordered enum inside a leaf, or one collection type for another under the same tag. The ABI records every field's type and collection kind, and `cargo near build` runs `storage diff` against the deployed wasm's ABI, so a swap, or a tag reused for a different type, is a build error.

**Cost.** Two bytes per field for tags and lengths under 128, under 1 Ggas per write for a typical struct. The ABI gains a record schema so clients and `cargo near` can decode records without the contract.

**Why it is in the first release.** A record encoding cannot be switched later without rewriting every stored record, which is the migration this design exists to remove. It freezes alongside the key format.

**Unknown tags are preserved.** Code that meets a tag it does not know keeps those bytes and writes them back through `get_mut` and `entry`, so rolling back to an older version cannot destroy data the newer one wrote. A blind `insert` replaces the record whole and drops them. The cost is one boxed slice per record, empty in the common case. A deliberately removed field is listed as `#[near(storage, reserved(3))]`; reserved tags are dropped on rewrite. Reusing a tag for a different type is a build error against the deployed ABI, since stale bytes would resurface under the new field: an audit found stale `locked` bytes resurfacing as `claimable`.

## Collections and cost bounds

One family, each type with a stated bound. Operations are counted, not estimated: a read is about 56 Ggas plus bytes, a write about 64 Ggas plus bytes and trie nodes, a remove about 53 Ggas plus bytes and nodes. An overwrite also pays 32 Mgas per byte of the value it replaces. Every bound below depends only on the caller's own key and value, and for a `Map` removal on the key that moves into the gap, never on a neighbor's value.

Which type to reach for:

- `Map<K, V>`: the default. A lookup table plus a key vector, so it has `len`, `page`, iteration, and a record holding one can be removed as a whole. A new key costs two small reads and four writes, about 0.5 Tgas over `LookupMap`, and stakes 305 bytes against 115 for a `u128` value (staked bytes below the cost table). `insert` always overwrites and returns whether the key was new (not `#[must_use]`, as in std). First-write-wins is `try_insert`, at the same cost: `require!(votes.try_insert(who, v).is_ok(), "already voted")` writes nothing when the voter already voted, where the old `require!(votes.insert(who, v), ..)` had overwritten the vote before failing.
- `LookupMap<K, V>`: lookup only, one write per insert, and `insert` returns `()` since it reads nothing. Entries can be removed one by one but never iterated, so a record holding one cannot be removed as a whole (see Removing a record). Choose it knowingly: the choice is permanent.
- `Vector<T>`: dense slots with push, pop, swap-remove and indexing.
- `Set<K>` and `LookupSet<K>`: the two maps with a unit value.
- `Lazy<T>`: one value at its own key, read only when touched. For large or rarely used fields.
- A map of maps is a `Map` whose value is a record holding a `Map`, as `Market` above. `GroupedMap` is deferred out of the first release; allowing a collection directly as a map value is the alternative to weigh later.
- `OrderedMap<K, V>`: sorted iteration and range queries, after the first release.

| Type | Keys it owns | get | insert new | update existing | remove | Removal cost scales with | Staked bytes per entry |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `LookupMap<K, V>` | `00` entries | 1 read; `contains_key` is 1 host call and no bytes; `at_mut` is 1 existence check, no read, no decode. For a value that owns collections a miss adds 1 host call on the flag key (see Bounded clearing) | 1 write, no existence read; `insert` returns `()`. `try_insert`, for any value, adds 1 existence check and writes nothing if present (`Err(Occupied)`); for a value that owns collections it is the only insert, and a miss adds a 2nd check, its flag key | 1 write plus the old value's bytes; `replace`, which returns the old value, adds 1 read | 1 read, 1 remove; `discard` skips the read, and so does a unit value; plus clearing any collections inside the value | removed value size, plus one remove per key the value's own collections hold | 1 record: key + value + 40 |
| `Map<K, V>` | `00` entries, `01` index, `02` key slots, `03` header | 1 read | 1 index read, 1 header read, 4 writes: entry, index, key slot, header; `insert` returns whether the key was new, from the index read. `try_insert` costs the same, the index read being the check, and writes nothing if present; values that own collections enter by `try_insert` or `entry(k).or_default()` (1 more read, the flag key, after the entry misses) | 1 write after a read in the same call; `insert` alone is 1 index read and 1 write | 1 entry read (skipped by `discard`), 1 index read, 1 header read, 2 removes for entry and index, 1 read and 1 remove of the last slot, 2 writes for the moved key and its index, 1 header write. Removing the last key skips the moved-key writes. Every removal bumps the revision in the header, which this header write already carries; emptying the map writes the header with length 0 instead of removing it, so the revision never resets, and the header goes only when the record owning the map is removed. `discard` of a value that owns collections adds 1 remove, its flag key | removed value size, own key size, and the size of the last key, which moves into the gap | 3 records: entry (key + value + 40), index (key + 4 + 40), key slot (slot key + borsh(k) + 40); plus one header record per map (key + 4 + 40, or key + 12 + 40 once a removal has bumped the revision) |
| `Vector<T>` | `00` slots, `03` header | 1 read | 1 header read, 1 write, 1 header write | 1 write | pop: 1 header read, 1 slot read for the returned value, 1 remove, 1 header write; swap\_remove: plus 1 read and 1 write of the last slot | last element size | 1 record: slot key + value + 40; plus one header record |
| `Set<K>`, `LookupSet<K>` | as `Map` and `LookupMap` with a unit value | same | same | same | same, minus the value read | key size | as `Map` and `LookupMap` with a zero-byte value |
| `Lazy<T>` | its own path | 1 read | 1 write | 1 write | 1 remove | value size | 1 record: key + value + 40 |
| `OrderedMap<K, V>` | `00` entries, `01` tree nodes, `03` header | 1 read | 1 write plus O(log n) small node writes | 1 write | 1 remove plus O(log n) small node writes | tree depth and key size | entry record plus its tree-node record |

**Staked bytes.** The runtime charges 40 bytes of overhead per record, so a new `Map` entry stakes three records plus a per-map header. For `accounts[alice].balances[usdt]` holding a `u128`, that is 115 bytes for the entry, 103 for the index and 87 for the key slot, 305 in all, against 115 in a `LookupMap`; the header adds 79 once per map. NEP-145 deposit math must use these bytes.

**Removing a record.** A value's collections live under child keys, so removing the record alone would orphan them. `remove` therefore requires `V: Removable`: every opaque leaf is, and a `#[near(storage)]` struct is when its collections can all be iterated (`Map`, `Set`, `Vector`, `Lazy`). The derive clears each collection before the record goes, one header read, then one slot read and three removes per `Map` entry; four when the entry's value itself owns collections, since a bulk clear does not read the entry and so also removes its flag key, in case that record was mid-clear. A struct holding a `LookupMap` or a `LookupSet` is not `Removable`, and `remove` on it does not compile, with the error naming the field and the fix: use `Map`, or keep the record and clear fields by hand. A v1 collection is not a storage value at all, so it cannot be nested in a record, and `remove` can never orphan one; at the root, where `remove` never reaches, it is a `#[key(n, legacy)]` field. (Before round 3 a `FungibleToken` inside a record compiled, and `remove` orphaned its balances.) Clearing is the one operation whose cost scales with what the record owns, so `remove_step(&k, n)` removes at most n entries per call and returns `Removal::Pending` while entries remain and `Removal::Done` once the record is gone, and the `bounded_work` lint flags an unbounded `remove` on a record with collections. Across calls the record is flagged, which makes writes below it panic until clearing finishes, since an attacker adding 600 orders per block would otherwise outrun `remove_step(&k, 500)`. The same rules govern the old entry in `Migrating`. Protocol ask A turns the clearing into one host call.

**Bounded clearing in detail.** The call is `Map::remove_step(&k, n) -> Removal` and `LookupMap::remove_step(&k, n) -> Removal`, with `#[must_use] pub enum Removal { Pending, Done }`: `Pending` means entries remain and the record is flagged, so call again in a later call; `Done` means the record is gone or was absent. The derive emits `Removable::clear_children_some(path, &mut budget)` beside `clear_children`. It is named `remove_step` and not `clear(n)` because it sits on the map, the same receiver as the existing whole-map `Map::clear()`, and Rust has no overloading; the operation also ends by removing the record, which only the map can do. The rules the prototype settled:

- The flag is a separate key, the record's key plus one `00` byte (the Flag segment under Keys are paths from the root). While a record is being cleared its bytes, unchanged, live at the flag key and its own key is absent. The move copies the stored bytes verbatim; it does not re-encode the record, which could add `default = ..` fields the stored record lacks (an old `Market` would gain `04 01 01`). Only a record changed earlier in the same call moves as its new encoding (review F3). So the record key's existence alone means "present and not being cleared", and record bytes never carry a flag; tag 0 in a record is always an error.
- Only map values with collections (`MAX_SUBPATH > 0`) consult the flag key, and only after a miss on the record key: `get` and `entry` then read it (1 more read), and `contains_key`, `LookupMap::try_insert` and `at_mut` check it (1 more existence check). `Map::try_insert` checks the index instead and pays nothing extra. A hit costs nothing extra, so `get_mut`, `entry` and `at_mut` on a present record cost what they did before clearing existed.
- While the flag is set, `get_mut`, `entry`, `at_mut` and `LookupMap::try_insert` on that record panic; `get`, `at` and iteration still read it. `Map::try_insert` returns `Err(Occupied)`, since the key is still indexed.
- The budget counts entries removed at any depth; `Lazy` values do not count. `n = 0` clears nothing and sets the flag.
- `replace`, `remove` and `discard` on a clearing record are allowed: they clear without a bound and remove the flag key. A blind `discard` of a record with collections removes both keys, 1 more remove, since it does not know which one holds the record; if the same call then puts a new record at that key (`try_insert`, `entry(k).or_default()`), the flush writes the record key and still removes the flag key, so an old mid-clear record cannot survive beside its replacement (review F1).
- A clear that fits in one call never touches the flag key. Per call: 1 read of the record (2 once flagged: the record key misses, then the flag key), the cost of clearing up to `n` entries, and on the first call that does not finish, 1 remove of the record key and 1 write of the flag key; the call that finishes removes the flag key. For storage staking, a flagged record holds one more key byte than it did, and nothing more, since its bytes move unchanged.
- `Map` drops the key from its key vector in the call that finishes.

Tests: `clear_across_two_calls_refuses_an_interleaved_insert`, `writes_below_a_clearing_record_panic_in_the_same_call`, `clear_that_fits_in_one_call_writes_no_flag`, `remove_step_budgets_across_calls`, `flag_key_is_dropped_by_replace_remove_and_discard`, `flag_key_is_the_record_key_plus_a_zero_byte`, `tag_zero_is_rejected_even_leading_and_empty`.

**`at_mut` is one existence check.** The first prototype put the flag in the record's bytes as a leading `00 00`, which `storage_has_key` cannot see, so `at_mut` had to read the record. Storage written by those builds must be reset: such a record fails to decode (`TagZero`) on `get`, `get_mut` and `entry`, but `at_mut` sees only that its key exists and allows writes below it. With the flag key, a present record key already says the record is not being cleared, and `at_mut` is back to one existence check with no read and no decode. Measured in the prototype, in reads, writes, removes:

| Method | Flag in the record | Flag key (now) |
| --- | --- | --- |
| Exchange `set_balance` | (3, 1, 0) | (2, 1, 0) + 1 existence check |
| Exchange `cancel` | (6, 3, 3) | (5, 3, 3) + 1 existence check |
| Exchange `fill` (`at_mut` then `get_mut` on an order) | (3, 1, 0) | (2, 1, 0) + 1 existence check |
| DAO `vote` | (7, 8, 0) + 1 existence check | (5, 8, 0) + 2 existence checks |

These equal the counts before clearing existed, and the DAO `vote` again lands a vote under a proposal without reading its 2,000-byte description. The cost moved to misses on map values that own collections, which pay 1 more host call for the flag key. Creating a record is such a miss: the first Exchange `deposit` for an account is (6, 8, 0), was (5, 8, 0), and `try_insert` of a record on a `LookupMap` is 2 existence checks, was 1. An `at_mut` on an absent record is 2 existence checks. Calls on existing records, such as a repeat `deposit` or a `place`, do not change.

**Two small keys instead of one big rewrite.** v1 `IterableMap` keeps each entry's key-vector index inside its value, so removing a non-last entry rewrites the moved key's whole value. v2 gives the index its own 4-byte slot: one extra small write on insert, and removal bounded by the caller's own value plus the moved key, never by a neighbor's value. The length gets its own 4-byte header key for the same reason: an insert never rewrites the parent record, and `at(&id).balances.len()` is one read. The last key moves into the gap, so iteration order is not insertion order after a removal.

&#91;embedded content: removal · v1 IterableMap vs v2 Map\]

**Pagination.** `page(after, limit)` reads the cursor key's index slot and the header, then walks the key vector: two reads, plus one key slot and one entry per item. If the cursor key was removed since the last page, it returns `Err(CursorGone)` instead of panicking, JSON views map the error, and the client restarts from a known key. A removal elsewhere between pages moves the last key into the gap, so a key can be skipped; cursor pages are exact only across calls with no interleaved removals. Each page therefore carries `revision`, a counter in the map's header that every removal bumps, read with the header the page reads anyway, so it costs nothing: a caller that needs an exact walk compares revisions across pages, and equal revisions mean no removal happened in between. The header is already written on every removal, so the counter adds no write there. Two costs are new: a map that has had a removal stores a 12-byte header instead of 4, and a map that empties keeps its header (length 0) rather than removing it, so the revision never resets to a value an earlier page carried; the emptying call writes the header where it used to remove it, and the key stays staked until the record owning the map is removed. On-chain batch jobs must not page a map that changes between calls, and offset paging can likewise skip a key after a removal behind the cursor. `iter().skip(from).take(limit)` costs nothing per skipped element, since the key vector is indexed, and is the shape for NEP-181 offset enumeration and for public views that page while other calls remove. Each map hop also costs one sha256 of the key, 4.8 Ggas measured against 56 for a read.

**Removal returns the value.** The host remove runs at flush, so `remove` reads the value it returns now, one read on top of the remove. The host charges 11.5 Mgas per byte of the removed value either way. `discard` skips the read and the decode.

**Measured on the real runtime.** near-contract-sim (nearcore 2.14.0-rc.2, protocol 87) gives the gas a node charges; full figures are in `GAS_MEASUREMENTS.md` on branch `proto/storage-v2-gas`. A new key in `Map` costs 0.48 to 0.53 Tgas more than in `LookupMap` and 0.28 to 0.30 more than in v1 `IterableMap`, flat from 1k to 100k keys: two more reads, three more writes and their trie nodes. Whole receipts at 100k keys, insert new: `Map` 1.80 Tgas, `LookupMap` 1.31, v1 `IterableMap` 1.52, v1 `LookupMap` 1.40; update existing: 1.42, 1.32, 1.43, 1.42.

**Entry hash width.** Sixteen bytes stays: eight saves 0.5 to 2 Ggas per operation, about 0.1% of a receipt, and touches identical trie nodes. `accounts[alice].balances[usdt]` is 59 bytes at 16 and 43 at 8. Storage is where eight would pay: a nested `Map` entry stores the hash in three keys, so eight bytes saves 40 bytes per entry (16 in the value key, which holds two hashes, 16 in the index key, likewise, and 8 in the key slot, which holds the parent's), about 400 NEAR of staked storage per million entries at 10^19 yoctoNEAR per byte. Sixteen is kept anyway. The width can never be widened once data is on chain, since every key would move. At eight bytes the birthday bound n²/2^65 is 3·10^-8 at a million keys in one map, small, but 0.03 at a billion. Correctness never depends on the hash, because the Borsh key follows it in every key, so a collision costs nothing but a longer shared trie prefix; and an attacker grinding a key prefix to shape the trie pays 2^b per hashed byte either way, so the width does not change that cost.

**Removal beside a large neighbor.** Removing a non-last key from v1 `IterableMap` costs 1.91 Tgas beside a 1-byte neighbor and 10.75 beside a 100 KiB one, as whole receipts; the operation alone is 0.87 and 9.71. v2 `Map` costs 2.12 Tgas in both cases, with byte-identical receipts.

**Per call.** The root read costs 89 Ggas on every call. A nested read below the root, the `balance_of` shape, costs 138 Ggas: one read and two sha256 at 4.8 Ggas per hop. An unchanged `&mut self` call writes nothing in v2, where v1 rewrites `STATE` for 92 Ggas.

`UnorderedMap` and `UnorderedSet`, with their free-list holes, do not come to v2, and `IterableMap`'s name does not either: the iterable type is the default here, so it takes the plain name. `OrderedMap` ships after the first release on the same key scheme; its tree compares decoded keys, so it needs no order-preserving encoding.

## The root is a record too

The root struct is one tagged record at the root key, read and decoded once at the start of every call and written once at the end of a mutating call, only if a scalar changed. That read is the one cost every method pays, so root scalars stay small and anything large goes in `Lazy`. Collections and `Lazy` fields are references out of it and cost nothing until touched. One rule covers every level: a struct is a record, a collection is a subtree. A call holds one root; the wrapper creates it, and the reference-returning `get` relies on that.

**What happens to `STATE`.** A v2 root lives at the root byte, not at `STATE`. The `#[near]` wrapper reads it before the call and flushes after, through a pair of load and store hooks on `ContractState` that carry no Borsh or `Default` bound, so a v2 root needs no `PanicOnDefault`. `Drop` is a no-op in release builds and, in tests, a debug assertion that the wrapper flushed, skipped while a panic is unwinding.

**Migrations that remain.** Changing a field's type, splitting a map, or moving data between paths still needs code. A `fn migrate(old: RawRecord) -> Self` hook, not yet built (the prototype's migrations are plain `#[init]` methods), receives the old root as raw tagged fields, so renaming a tag or converting one field is a few lines. Versioned roots via an enum stay possible but are no longer the default advice.

## One cache, one flush

In v1 every collection has its own cache and flushes in `Drop`, in whatever order the fields drop, so a method's write pattern depends on code structure. In v2 there is one cache per call, keyed by storage key bytes. A read fills it, a write marks the key dirty, a remove marks it deleted. Slots are boxed and never freed within a call, so `get` can return a reference into the cache, and a loop over ten thousand entries holds ten thousand decoded values in memory, which is what the `bounded_work` lint is for.

The `#[near]` wrapper flushes at the end of the method: dirty keys once each in key order, then the root record last, so it reflects everything else, then it drops the cache, so a v1 collection held as a root field writes in its own `Drop` after the v2 keys. A balance updated ten times is one write. An insert then a remove of a key the call has already seen to be absent costs nothing. A view method cannot write: its handles come from `&self`, so the compiler stops it.

&#91;embedded content: one call through the cache · interactive\]

The same cache gives exact storage accounting. Every write and remove passes through one place, so after the flush the SDK knows the call's byte delta, including the 40 bytes the runtime charges per record, and exposes it as `env::storage_delta()` to a post-flush hook and to the storage-accounting guard; both ship in the first release (rollout step 5). It cannot be known earlier, because the old length of an overwritten key only comes back from the host. Inside the method body `storage_usage()` is pre-flush, so inside a v2 mutating method the mock treats reading it as an error, not a warning: the NEP-145 before-and-after pattern measures zero there, and registering for 1 yocto could drain the contract. NEP-145 deposit math uses the per-entry bytes under Collections and cost bounds.

**What the prototype does.** Under `unit-testing`, `env::storage_usage()` panics while the v2 cache holds unflushed changes (any slot not clean), with a message pointing at `storage_delta()`. The guard keys on cache state, not on a "v2 call active" flag, so views and code between calls never trip it (the ft-v2 migration test reads `storage_usage()` right after a view), and any "after" read in a method that wrote does. The gap: a change to a root scalar alone does not trip it, because the root is encoded only at the flush. `Receipt::storage_delta()` and `testing::storage_delta()` give the post-flush value at key + value + 40 bytes per record, and removes record the replaced length when the key existed; in the token test the mock's delta equals the mocked blockchain's `storage_usage()` delta. Tests: `storage_usage_inside_a_v2_mutating_method_panics`, `storage_delta_is_the_post_flush_storage_change`. The on-chain `env::storage_delta()` is not built; inside a method body it needs host support (Protocol asks, E).

Less code is a goal, not yet a result. The prototype's v2 wasm is larger than v1: 23 KB larger in the gas run, and 129 KB against 184 KB in the fungible-token migration run, about 60 Ggas of extra contract loading per call. One cache in place of seven, and a key builder of LEB128 and sha256 in place of a Borsh serializer per prefix enum, are meant to close that gap. `Map::insert` also hashes the key three times where once would do, about 10 Ggas, a known prototype inefficiency.

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

**What the format does not protect against.** Direct `env::storage_write` under the root byte, which the simulator flags. A v1 collection kept at the root whose prefix is empty or starts with the root byte: the one stated exception to guarantee 1, which `migrate` refuses as Coexistence describes. And it does not make a single storage operation cheaper; it removes the paths that touched other entries.

## Coexistence and migration from v1

Nothing in `store` or `collections` changes, and no default hasher moves. Moving deployed data silently would be worse than the bug.

**New contracts** use `near_sdk::state` only and never see a prefix.

**Deployed contracts** adopt it in three steps, each optional and each a normal upgrade.

- **Move the root.** Switch to the v2 layout and write a one-time `migrate` that reads the old `STATE` blob into the new root record and removes `STATE`. On the real runtime, against the unmodified fungible-token example, migrate costs 1 has_key, 1 read of `STATE`, 1 remove and 1 root write; every other v1 key stays byte-identical, and all 300 balances read back. Existing v1 collections stay as fields of the new root, marked `#[key(n, legacy)]`, with their old keys. The two v1 families differ: `near_sdk::store::*` writes at drop, after the v2 flush, while `near_sdk::collections::*`, which near-contract-standards' `FungibleToken` and `NonFungibleToken` use, writes immediately in the method body, before the flush. They are allowed only at the root, as `legacy` fields: nested inside a record they do not compile, because no v1 type, and no standards type holding one such as `FungibleToken`, implements the v2 value trait. Each v1 prefix kept at the root must be non-empty and must not start with the root byte, and `migrate` checks it. The big maps never move; one record is rewritten. A v2 transfer costs more than v1, 1.88 against 1.67 Tgas in steady state and 2.20 on a sender's first touch of the new field, because the wasm is larger and the new field adds a read and a write; the `STATE` rewrite goes away.
- **Add new fields as v2 types.** Every field added from now on is path-derived and collision-free. Most contracts stop here.
- **Rekey an old collection when it is worth it.** `Migrating<V1, V2>` wraps an old and a new map: reads try the new path and fall back to the old, two reads per miss; writes go to the new path and delete the old entry. Every key is on at most one side, and nothing trusts that rule blindly: `migrate_batch` drops an old entry whose key is already on the new side instead of overwriting the newer value (free on a v2 `Map` target, 1 `storage_has_key` per entry on a `LookupMap` target), and `remove` deletes from both sides (a key that already moved pays 1 old-side remove more, 1 read for a v1 `TreeMap`). `new_side()` and `old_side()` are read-only; the mutable `new_side_mut_unchecked()` and `old_side_mut_unchecked()` bypass the rule and say so. Going to `Migrating` takes a new tag plus `reserved(old)`, and the field must be `required`. On iterable v1 types (`collections::TreeMap`, `store::IterableMap`) `migrate_batch(n)` moves entries across calls: it drained 240 entries in 5 calls, but TreeMap rebalancing made the first batch cost 1,437 reads, so size batches small. The wrapper then comes off cleanly, though the stale v1 prefixes stay in the root record forever under reserved tags. On a v1 `LookupMap`, `migrate_batch` is a compile error (E0277) with a message saying why, so the lazy path is permanent unless an indexer supplies the keys: every miss costs two reads, and the old digest keys remain a collision target as long as any raw-key writer remains.

**Upgrade shape.** Deploy and `migrate` must be one transaction. v2 code deployed without running migrate bricks every call with `cannot decode root: Missing { tag: 1 }` until migrate runs, and any public `#[init]` would be callable in that window.

**The sharpest edge: the standards.** near-contract-standards' maps are concrete fields of `FungibleToken` and `NonFungibleToken`, so rekeying means forking the standard. Each moved key grows from 17 to 35 bytes, about 180 NEAR of staked storage per million holders, paid by the contract, and new registrations keep paying the v1 bond (125 bytes) for a 143-byte record. Independent of the fork, the standards cannot adopt v2 at all until `env::storage_delta()` ships: `FungibleToken::new` measures its storage bond with `storage_usage()` before and after inside init, which the v2 mock now refuses, and there is no on-chain `env::storage_delta()` yet (Protocol asks, E).

**The root byte.** All v2 keys start with one byte. A v1 prefix starting with that byte, or an empty v1 prefix, could alias a v2 key. The migration round showed that a default byte and a state scan are not enough: an empty v1 prefix with caller-chosen byte keys let a caller write a v2 key under tag 1 after about 256 hashes, and `String` keys need about 2^15 times more. `AccountId`-keyed v2 maps are immune. So `migrate` checks the v1 prefixes kept at the root: each must be non-empty and must not start with the root byte. The default byte stays outside the range small `BorshStorageKey` enums produce and configurable per contract, and the simulator's scan of real state stays as a second check. `STATE` cannot collide; its first byte is `S`.

**Hardening v1 meanwhile.** Independent of v2, a `Namespaced` hasher producing `prefix ++ sha256(key)` fixes the collision shape on new fields with a one-type-parameter change, since every hashed key then carries its enum tag. It is a sixty-line, non-breaking addition to a sealed trait, and the v2 scheme in miniature.

## Tooling

The layout is machine-readable, so most safety checks become build artifacts rather than review items. In the order they ship:

**Storage schema in the ABI, from the first release.** The derive that builds keys also emits a `storage` section into the ABI the contract already embeds: every path from the root, its tag, collection kind, key and value schemas, the record format version and the root byte. It costs nothing at runtime, and everything below reads it.

**Checks in near-contract-sim.** The simulator runs the contract on the real runtime, so it sees real keys and real gas. Given the schema, it asserts that every write lands under a declared path and that no v1 prefix starts with the root byte or is a prefix of another live v1 prefix. For v1 code it flags the collision shape directly: a raw-key collection writing 32-byte keys in a contract that also writes digests. That check would have failed hot-bridge's own tests. The unit-test mock keeps only a counting receipt, so `cargo test` stays fast.

**Gas receipt and gas-bound tests.** The mock counts reads, writes, removes and bytes per key per call; the simulator reports the same receipt in real Tgas. `#[gas_bound(reads = 1, writes = 2)]` asserts the counts in a unit test, and the SDK's CI runs one per operation per type, so the cost table above is checked rather than promised. A `bounded_work` lint, opt-in at first, warns on `iter()` without `take` or `page` inside a `#[near]` method.

**cargo-near commands, next version.** `storage check` validates the schema and the v1 prefix set for overlap, `storage decode` prints state as paths and typed values from a dump or RPC, and `storage diff` lists paths whose keys or types changed between the deployed wasm's ABI and the new build, the input to a migration. `cargo near build` runs `storage diff` against the deployed ABI, not the previous local build, and fails hard on a newly added `required` tag, a tag reused for a different type, or a type swap the record cannot see; that check ships in the first release. There is no lockfile; the deployed wasm embeds its ABI.

**Asks to cargo-near and near-abi.** Two of these checks live outside the SDK, and the prototype builds neither: the `storage diff` hard fail against the deployed ABI (a newly added `required` tag, a type swap the record cannot see), and the build error on a tag reused for a different type. Both need near-abi to carry the storage section and cargo-near to fetch the deployed wasm's ABI and compare. Until they ship, the prototype pins the tag-reuse hazard with `hazard_reusing_a_removed_tag_resurrects_its_stale_bytes`, which shows a reused tag reading a removed field's stale bytes, and covers a late `required` tag at decode with `field_added_as_required_fails_an_old_record_naming_the_tag`.

## Protocol asks

Everything above runs on today's host functions. The protocol cannot make moving bytes cheaper, but it can let contracts avoid moving them. A is proposed; B to E are for discussion, and E gates standards adoption. Runtime-team review of this section is requested before the proposal goes beyond NEAR One.

**A. Metered range iteration, as a NEP.** The trie is sorted by key, and `storage_iter_prefix`, `storage_iter_range` and `storage_iter_next` still exist in nearcore as stubs returning `Deprecated`. A revived version takes a start cursor, an end bound and a page limit, and charges per trie node visited plus per byte returned, with a hard page cap to bound proof size. This key format is built for it: every map's entries sit contiguously under `ns ++ 00`, so `Map` collapses to `LookupMap` plus a header, removal is one `storage_remove`, and cursors are plain keys.

**B. Conditional writes.** `storage_write_if(key, expected_hash, value)` writes only if the current value's hash matches. Every trie leaf already holds `ValueRef { length, hash }`, so the check needs no value read. This makes callback guards cheap for large values.

**C. Content-addressed values.** Leaves already reference values by hash, and values over 4,000 bytes are kept out of the inlined flat-storage entry. Exposing that gives deduplication and a `storage_move` charged per key byte rather than per value byte. Useful for large metadata and migrations; not needed for the collections above.

**D. Storage payment in the runtime.** Minimal version: apply a call's attached deposit to its storage delta and refund the rest, so users cannot drain a contract by filling its storage. Full version: a payer per key, refunded on delete, which adds per-key state and needs its own cost model.

**E. An on-chain `env::storage_delta()`.** After its flush the SDK can compute the v2 share of a call's byte delta. Inside the method body it cannot, because the old length of an overwritten key comes back only from the host, and v1 collections write around the cache. near-contract-standards needs the in-body value: `FungibleToken::new` measures its storage bond with `storage_usage()` before and after inside init, which the v2 mock refuses. The ask is host support for a call's storage delta so far; its exact shape is for the runtime team. Not built in the prototype; the mock has `Receipt::storage_delta()`.

Callback guards follow in a later phase as an explicit API: a guard object created at the top of a method and settled on return. Storage accounting ships in the first release, as One cache, one flush describes.

## Rollout and first PRs

`state` ships behind the `unstable` feature beside `store` and `collections`, and v1 keeps working throughout.

- **Harden v1 now.** Two small additive PRs: the `Namespaced` hasher, and a mock-time check that flags a raw-key collection writing 32-byte keys in a contract that also writes digests. The hasher goes first because it fixes the collision shape on new fields today.
- **Freeze the formats.** A two-page spec: the root byte, the four segment kinds, the entry hash width, the record encoding, the format version, the key length rule and the `Removable` semantics. Reviewed before any `state` code merges, because none of it can change once data is on chain.
- **Encoding module and schema derive.** The key builder, the record encoder, the `StorageKey` and `Removable` traits, the `#[key(n)]` and `#[near(storage)]` derives with their view types, and the ABI storage section. No collections yet. The harnesses for guarantees 1 and 6 ship here.
- **Collections.** `Map`, `LookupMap`, `Vector`, `Lazy`, `Set`, `LookupSet` and the `at` accessors, each with its gas-bound test in CI, and a public `state::testing` harness with the counting receipt, since every contract's tests want it.
- **Root integration, migration and accounting.** The v2 `contract_state` layout, the wrapper's flush, the `migrate` hook with its v1-prefix check, v1 collections as root fields, `Migrating`, the accounting guard and `env::storage_delta()`, which standards adoption waits on (Protocol asks, E), and `storage diff` in `cargo near build` against the deployed ABI, the only check for silent type swaps, reused tags and late `required` tags. The load and store hooks move a Borsh bound off `ContractState`'s provided methods, which is visible to a downstream override that restated it; the SDK has none, and the PR says so.
- **Tooling.** The near-contract-sim checks, then the three `cargo near storage` commands.
- **`OrderedMap`**, on the same key scheme.
- **Range-iteration NEP**, written in parallel with steps 3 to 6 and taken to the runtime team.

Steps 1 to 5, storage accounting included, are the first release. Callback guards, the NEP-145 derive and `GroupedMap` follow once the collections have shipped.

**What the prototype showed.** Branch `proto/storage-v2` holds every collection above, the views, the three macros, the `Exchange` contract, and three contracts written against the rustdoc alone: a NEP-141 token with storage deposits, an NFT with enumeration and approvals, and a DAO with per-proposal votes. All compile and pass. A counting host confirmed the key lengths, the cost table as corrected here, neighbor-independent removal, the flush order and the pagination formula. The usability pass set the names, `at_mut`, `page_with` and `Lazy::new`. The prototype is behind `unstable` and has no Kani or fuzz harnesses. Test counts after API round 3 and the D9 review fixes: lib 580, doctests 346 plus 4 standalone `compile_fail`, state_v2_exchange 36, state_v2_ft 16, state_v2_nft 17, state_v2_dao 15, state_v2_migrate 5, ft-v2 16 plus 2 doctests, near-sdk-macros 49, sim 2 (not rerun); the wasm32 check passes with no warnings. `PROTOTYPE_NOTES.md` and `USABILITY_NOTES.md` on the branch list every finding.

**Migration round, on the real runtime.** The unmodified fungible-token example, with 300 funded accounts and a 100 KiB icon, went v1 to v2 (move the root) to v3 (rekey) on nearcore's runtime through near-contract-sim. `migrate` cost 1 has_key, 1 read of `STATE`, 1 remove and 1 root write; every other v1 key stayed byte-identical, and all 300 balances read back after deploy and migrate in one transaction. A v2 transfer costs 1.88 Tgas steady and 2.20 on a sender's first touch of the new field, against 1.67 for v1: the 129 KB to 184 KB wasm adds 60 Ggas of loading, plus more wasm work and the new field, while the `STATE` rewrite goes away. `Migrating` costs two reads per miss; the first transfer while moving took 7 reads, 6 writes and 2 removes (2.18 Tgas), later ones 4 reads and 3 writes (1.88). `migrate_batch` drained 240 entries from a `collections::TreeMap` and a `store::IterableMap` in 5 calls, but TreeMap rebalancing made the first batch cost 1,437 reads, so batches must be small. On a v1 `LookupMap` it is a compile error (E0277) with a clear message. The wrapper came off cleanly, but the stale v1 prefixes stay in the root record under reserved tags. Going to `Migrating` takes a new tag plus `reserved(old)`, and the field must be `required`. Rekeying the standards' accounts map meant forking `FungibleToken`: each moved key grows from 17 to 35 bytes, about 180 NEAR of staked storage per million holders, paid by the contract, and new registrations keep paying the v1 bond of 125 bytes for a 143-byte record. Not built: the `migrate(old: RawRecord)` hook, since v3's migrate is a plain `#[init(ignore_state)]`; and `Migrating::get` returns an owned value (`V: Clone`).

**API round (Oct 17).** The prototype applied decisions D2 to D9; each has a line in the `state` module docs.

| Decision | What the prototype does | Tests |
| --- | --- | --- |
| D2 | `#[key(n, default = expr)]`: a missing tag evaluates `expr` lazily, in decode and in the generated `Default`, and the type needs no `Default`. `RecordError::Missing` names the tag and the fix. | `key_default_expr_fills_missing_tags`, `field_added_with_default_decodes_an_old_record`, `field_added_as_required_fails_an_old_record_naming_the_tag` |
| D3 | `GroupedMap` is `#[doc(hidden)]` with a deferral note. Exchange uses `markets: Map<MarketId, Market>`. | `markets_place_and_cancel`, `market_record_scalars_and_orders`, `cancel_in_an_absent_market_panics` |
| D4 | No change: unknown tags were already preserved and `reserved` tags dropped. | `unknown_tags_survive_get_mut_and_entry_reserved_tags_are_dropped`, `hazard_reusing_a_removed_tag_resurrects_its_stale_bytes` |
| D5 | The mock's `storage_usage()` guard and `Receipt::storage_delta()`, under One cache, one flush. | `storage_usage_inside_a_v2_mutating_method_panics`, `storage_delta_is_the_post_flush_storage_change` |
| D6 | `insert` always overwrites and needs `V: Leaf`, a blanket over Borsh that the derive also emits when no field owns storage (round 3: no Borsh blanket any more, see below). `insert_new(k, v) -> &mut V` panics if present (round 3: replaced by `try_insert`). `Map::insert -> bool`, `LookupMap::insert -> ()`. The rejected call is E0277 with the fix in the note. | a `compile_fail,E0277` doctest, `insert_new_returns_the_bound_record`, `insert_new_over_existing_record_panics` |
| D7 | `at_mut(&k) -> Option<NameAtMut>` on `Map` and `LookupMap`. | `at_mut_on_an_absent_record_is_none`, `set_balance_on_an_absent_account_panics` |
| D8 | `page`, `page_with`, `keys_page` and `Set::page` return `Result<_, CursorGone>`; `CursorGone: AsRef<str>`, so a `#[handle_result]` view returns it as is. | `page_with_removed_cursor_returns_cursor_gone`, `offset_paging_skips_a_key_after_a_removal_behind_the_offset` |
| D9 | `remove_some(&k, n)` (round 3: `remove_step`) and the clearing flag, under Collections and cost bounds. | listed there |

Where the decisions were silent, the prototype chose, and these are open to change:

- `default`, `default_fn` and `required` are mutually exclusive (a derive error); `default_fn` must be a path, not a closure or call, and a function that is not a `fn() -> T` is E0308 at the path; a `default` on a collection field is a post-monomorphization `const` assert.
- The `Market` scalars are `tick_size: u128` and `next_order_id: OrderId`, at tags 2 and 3, with `orders` at tag 1. The Exchange above is the prototype's text.
- The D5 guard keys on unflushed cache state, with the root-scalar gap described under One cache, one flush.
- `Lazy::set` on a record that owns collections is still a runtime insert-new check, not a trait bound. `MigrationTarget` requires `V: Leaf`, since values migrated from v1 are always Borsh.
- `keys_page` and `Set::page` also return `Result`, for consistency.
- The bounded clear is `remove_some(&k, n)` (round 3: `remove_step(&k, n) -> Removal`), with the rules under Collections and cost bounds.

The clearing flag first went in the record's bytes, which made `at_mut` read the record. It now has its own key, and `at_mut` is back to one existence check (the table under Collections and cost bounds).

**API round 3 (Oct 17).** The prototype applied round-2 decisions T1 to T7.

| Decision | What the prototype does | Tests |
| --- | --- | --- |
| T1 | `insert` stays an overwrite: `Map::insert -> bool` with `#[must_use]`, `LookupMap::insert -> ()`. `insert_new` is renamed `try_insert(k, v) -> Result<&mut V, Occupied>` and exists for every value type; `Occupied` is a unit struct, so `.expect(..)` needs no `Debug` on the value. Records that own collections still have no `insert`. `Map::try_insert` costs what `insert` costs; `LookupMap::try_insert` adds 1 existence check. A record being cleared is `Occupied` on `Map` and a panic on `LookupMap`, as before. The Exchange `set_balance` line now reads `let _ = ...insert(..)`. | `try_insert_returns_the_bound_record`, `try_insert_over_existing_record_is_occupied`, `try_insert_on_leaf_values_keeps_the_first_write`, the `compile_fail,E0277` doctest on `LookupMap::insert` | Not built: the deployed-ABI `storage diff` checks (Tooling, an ask to cargo-near and near-abi), the on-chain `env::storage_delta()` (Protocol asks, E, an ask to nearcore), and a trybuild `.stderr` snapshot for the D6 error; the doctest checks E0277 only.
| T2 | No code change. The accessor rule is one sentence under What the developer writes: `at*` is path arithmetic plus at most one existence check and never decodes, `get*` reads and decodes. | none |
| T3 | `Map::remove_some(&k, n) -> bool` and the `LookupMap` twin are `remove_step(&k, n) -> Removal`, with `#[must_use] pub enum Removal { Pending, Done }` (plus `is_done()`). The panic on a write below a clearing record names `remove_step` and `Removal::Done`. Costs unchanged. | `remove_step_budgets_across_calls` and the D9 tests, asserting `Removal` values |
| T4 | `Page` and `KeyPage` gain `revision: u64`, and `Map::revision()` reads it: a counter in the map header that every removal bumps (`clear_some` once per call). The header is `MapHeader { len, revision }`, stored as the old 4-byte length while the revision is 0 and as 12 bytes after. The header was already written on every removal except the one that empties the map, which removed it; that call now writes `len = 0` and keeps the key, so the revision never resets. Same op count, a write in place of a remove, and the key stays staked (key + 12 + 40) until the owning record is removed, when `clear_children` deletes it. DAO `finalize` (33, 0, 44) is now (33, 7, 37); `remove_member` (29, 3, 40) is now (29, 8, 36), one more op for a header an earlier call left behind. | `page_revision_changes_only_across_a_removal`; the DAO and NFT no-leak tests now allow exactly the emptied maps' headers |
| T5 | No code change; the hash stays 16 bytes. The Entry hash width paragraph gains the storage comparison: 40 bytes per nested entry and about 400 NEAR per million entries saved at 8 bytes, and why 16 is kept. | none |
| T6 | `#[key(n, default = expr)]` takes a const expression only: the derive emits `const __KEY_DEFAULT: T = expr;` inside the decode, spanned at `expr`, so `env::predecessor_account_id()` is E0015 at the expression. `default = true` (Exchange) works unchanged. Consequence: `AccountId` and `vec![..]` have no const form, so a later-added `AccountId` field is `Option<AccountId>`; the prototype's D2 tests used `"treasury.near".parse().unwrap()` and `vec![1, 2]` and were moved to const defaults. No fn-path escape hatch; a possible later addition. | a `compile_fail,E0015` doctest and a passing const doctest in the `state` module docs; `key_default_expr_fills_missing_tags`, `field_added_with_default_decodes_an_old_record` |
| T7 | No blanket impl over Borsh: `Value`, `Leaf`, `Removable` and the view traits are blanket over a new marker `BorshLeaf`, implemented for the integers, `bool`, `()`, `String`, `AccountId`, `NearToken`, `Gas`, `PublicKey`, the `json_types` integers and `Base64VecU8`, and `Vec`, `Option`, `[T; N]`, `BTreeMap`, `BTreeSet` and tuples up to four of leaves. A user type opts in with `#[derive(Leaf)]` (from `near_sdk::state`; separate from `#[near(storage)]`, which makes a tagged record). `StorageKey` now requires `BorshLeaf`. v1 types implement nothing, so a v1 collection, a shadowed `use near_sdk::store::LookupMap`, or a `FungibleToken` inside a record is E0277 "`T` is not a storage v2 value". `#[key(n, legacy)]` is new: root only, required, Borsh-encoded outside the trait; the derive also rejects a v1 collection name at the root without it. Root bytes are unchanged. `Option<T>` is a value only for leaf `T`, not for records. | `compile_fail,E0277` doctests on `Value` (shadowed import; a Borsh struct holding a `collections::LookupMap`) and in ft-v2 (`FungibleToken` in a record), `legacy_root_field_is_its_borsh_bytes_and_required` |

**Gas round.** The same simulator measured the cost table against v1 on branch `proto/storage-v2-gas`; Collections and cost bounds carries its figures.

## Open questions

Each row states the proposed answer and the evidence. Disagreement with an answer, or evidence a row is missing, is the most useful comment this document can get.

| Question | Why it matters | Proposed answer |
| --- | --- | --- |
| Entry hash width, 16 or 8 bytes | 8 saves 8 bytes per key on every operation; 16 matches Substrate and leaves more margin against trie shaping | Decided on measurement: sixteen. Eight saves 0.5 to 2 Ggas per operation, about 0.1% of a receipt, and touches identical trie nodes; `accounts[alice].balances[usdt]` is 59 bytes at 16 and 43 at 8. |
| Unknown tags on rewrite, drop or preserve | Preserving keeps data written by a newer version across a rollback, at the cost of carrying unknown bytes in memory | Decided: preserve through `get_mut` and `entry`, drop on a blind `insert`. A removed field's bytes persist until `reserved(n)` lists its tag, and reusing a tag for a different type is a build error against the deployed ABI. |
| How the macro selects v2 | Presence of `#[key]` is implicit; a flag is one more thing to type | Explicit `contract_state(layout = v2)`. The layout is permanent and should be visible at the declaration. |
| `Map` insert at four writes | One more small write than v1 per new key, three more than `LookupMap`, in exchange for bounded removal | Accept: `Map` stays the iterable default and `LookupMap` is the opt-out. Measured, a new key costs 0.48 to 0.53 Tgas more than in `LookupMap` and 0.28 to 0.30 more than v1 `IterableMap`, and stakes 305 bytes against 115 for a `u128` balance. |
| Default root byte | Must avoid small enum discriminants and ASCII prefixes in deployed v1 contracts | 0xF0, configurable per contract. Every v1 prefix in the SDK's examples and standards is a small `BorshStorageKey` variant or a short ASCII string, none starting above 0x7F. The simulator asserts no overlap. |
| `GroupedMap` | Order books per market are a map of maps | Deferred out of the first release. A `Map` of records holding a `Map` covers it; allowing a collection directly as a map value is the alternative worth considering later. |
| `Lazy<T>` in the first release | Needed for large root fields once the root is a single record | Yes. It is the smallest type in the set. |
| Key length limits | The protocol rejects keys over 2,048 bytes, and a nested path can get there with two long map keys | A declared maximum per key type, summed along each path by the derive, which refuses a schema that could exceed 2,048 bytes. Strings and byte vectors default to 512 bytes. The mock warns on any key over 256 bytes, since a 1 KiB key adds 70 Ggas to every write. Part of the step-2 spec, because a cap cannot be lowered once data is on chain. |
| Writes to `env::storage_write` under the root byte | Bypasses every guarantee above | The simulator flags it. The SDK documents a `clippy.toml` snippet that denies `env::storage_write` and `env::storage_remove` in crates using `state`; clippy reads it from the contract's workspace, so the SDK cannot ship it. |
| Name of the module | `state` reads well beside `store` but is close to `contract_state` | `state`. The module already exists and holds the `ContractState` trait every `#[near]` wrapper calls, so the collections join it. `storage` would clash with `env::storage_*`; `store2` says nothing. Confirm at step 2. |
| Names | The usability pass hit `Vec` shadowing `std::vec::Vec`, and `Map` as the lookup-only type steering new schemas into a permanent no-iteration choice | Decided: `Map` and `Set` are the iterable types, `LookupMap` and `LookupSet` the lookup-only ones as in v1, `Vector` as in v1. |
| Fields without `Default` | `required` keeps the field's type; `Option` makes the absence visible in code | Decided: `#[key(n, default = expr)]` (const), `#[key(n, default_fn = path)]` (a pure `fn() -> T`, called on every decode where the tag is absent), or `required`, never a forced `Option`. `required` only for fields present since the first write; `cargo near build` hard-fails a newly added `required` tag against the deployed wasm's ABI. |
| v1 and v2 share `LookupMap`, `LookupSet` and `Vector` | `use near_sdk::store::LookupMap;` next to `use near_sdk::state::*;` silently made every `LookupMap` in the file the v1 type, records included, with no clash and no warning | Decided (round 3, T7): a positive bound instead of new names. Every field type must implement the v2 value trait; no Borsh blanket exists, so v1 types implement nothing and the shadowed import is a compile error at the field ("is not a storage v2 value", with a note naming `#[key(n, legacy)]` and the shadowing). v1 collections are allowed only as root fields marked `#[key(n, legacy)]`; plain Borsh structs opt in with `#[derive(Leaf)]`. |
| Name of the bounded clear across calls | `clear(n)` would sit on the map beside the existing `Map::clear()` | Decided (round 3): `remove_step(&k, n) -> Removal`, a `#[must_use]` enum `{ Pending, Done }` in place of the bool, whose direction readers misread. The map is the receiver, `clear(n)` would collide with `clear()`, and the operation ends by removing the record. |
| `at_mut` cost with the clearing flag | With the flag in the record's bytes, which an existence check cannot see, `at_mut` had to read the record: `set_balance` 3 reads, `cancel` 6, DAO `vote` 7 and rereading the 2,000-byte description | Decided: the flag is its own key, the record's key plus `00`, and a record being cleared moves there, leaving its own key absent. A present record key means "not being cleared", so `at_mut` is one existence check with no read: `set_balance` (2, 1, 0) + 1 check, `cancel` (5, 3, 3) + 1, DAO `vote` (5, 8, 0) + 2. Misses on map values with collections pay 1 more host call for the flag key, so creating such a record costs 1 more (first `deposit` 6 reads, was 5). |
