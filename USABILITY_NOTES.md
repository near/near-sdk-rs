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
