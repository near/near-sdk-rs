# near_sdk::state usability notes (draft)

## 1. Compile errors

## 2. Things I guessed

## 3. Awkward code

## 4. Naming and discoverability

- No public rustdoc for `#[near(storage)]`, `#[key(n)]`, `#[key(n, required)]`, `reserved(..)`,
  `contract_state(layout = v2, root = ..)`. `state/mod.rs` has no module doc.
- Map/Set/GroupedMap not Removable (found by grepping `Removable for`).

## 5. What was good

## 6. Status

### scratch: FT
- compiled first try (zero compile errors). Patterned on exchange closely.
- ft_transfer rwd (3,2,0)+1 has_key: contains_key(receiver) then get_mut(receiver) = has_key + read; has_key does not warm the cache.
- storage_unregister: get() then remove() = 1 read only (cache), good. Force: (2,1,1).
- storage_deposit dup: no writes.
- 152 bytes per registration (53-char account) via use_env_host + env::storage_usage; counting mock has no storage_usage. testing_env! resets storage_usage (harness trap, not SDK).
- Guessed: record with two scalars vs Borsh struct leaf as Map value -- no guidance which to use.
- Guessed: panic mid-method then begin_call -> cache dropped, nothing persisted (confirmed by test).
- Promise in tests: `.detach()` needed? (used it; v1 contracts don't)

### scratch: NFT
- compiled first try again. Disjoint-field borrow (token &mut from self.tokens live while self.tokens_per_owner mutated) fine.
- nft_transfer: guessed (7,7,3), real (9,8,3). Off because group remove() reads the () value; discard -> (8,8,3). clear() on empty approvals reads header.
- IterSet::remove == discard, but no GroupedSet so GroupedMap<_,_,()> users pay a read with remove().
- burn with 3 approvals (11,2,17), zero orphans, Lazy metadata removed without being read.
- Lazy nested in record: set before insert -> runtime panic "not bound"; must insert then get_mut().metadata.set(). Message good.
- Cursor page panics if cursor token transferred away: in a public view, a frontend gets a failed view call.
- swap-remove reorders owner list: [t0,t1,t2,t3] transfer t1 -> [t0,t3,t2].
- Offset pagination: U128 -> u32 casts, key_at loop by hand. No `page_at(offset, limit)`.
- nft_token: get() returns &Token whose collections are bound -> token.approvals.iter() works. at() vs get(): needed get for owner_id.
- IterSet has no key_at/keys_page -> can't use IterSet for owner lists; Map value can't be collection -> GroupedMap.
