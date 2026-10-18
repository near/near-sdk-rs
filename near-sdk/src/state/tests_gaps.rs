//! Tests closing cargo-mutants gaps listed in PROTOTYPE_NOTES.md ("Mutation testing", soundness
//! round of Oct 17). Each test names the mutants it kills.

use super::key::{DEFAULT_ROOT, ENTRY_HASH_LEN, Path, kind};
use super::root::__private as p;
use super::testing::{self, end_call};
use super::tests::{Account, Ex};
use super::*;
use crate::AccountId;
use std::vec::Vec as StdVec;

fn alice() -> AccountId {
    "alice.near".parse().unwrap()
}
fn bob() -> AccountId {
    "bob.near".parse().unwrap()
}
fn at<C: Default + Value>(tag: u32) -> C {
    let mut c = C::default();
    c.bind(&Path::root(DEFAULT_ROOT).field(tag));
    c
}

// ---------------------------------------------------------------------------------------
// 1. `cache::has_pending_writes`: the view-call rule, driven negatively
// ---------------------------------------------------------------------------------------

/// Stores alice (nonce 1, one balance) and starts a fresh call with the root loaded.
fn ex_with_alice() -> Ex {
    testing::use_mock();
    let mut ex: Ex = load_root();
    let a = ex.accounts.entry(alice()).or_default();
    a.nonce = 1;
    *a.balances.entry("usdt".into()).or_default() = 5;
    flush_root(&ex);
    drop(ex);
    testing::begin_call();
    load_root()
}

/// A view that only reads (record get, nested get, has_key, absent key) passes, and so does
/// a `get_mut` that leaves the value unchanged (the slot is touched but its bytes match, so
/// the flush would do nothing). Kills `has_pending_writes -> true` and the deleted `!`.
#[test]
fn gaps_view_that_only_reads_passes_the_view_check() {
    let mut ex = ex_with_alice();
    assert_eq!(ex.accounts.get(&alice()).map(|a| a.nonce), Some(1));
    assert_eq!(ex.accounts.at(&alice()).balances.get(&"usdt".into()), Some(&5));
    assert!(ex.accounts.contains_key(&alice()));
    assert!(!ex.accounts.contains_key(&bob()));
    assert!(ex.accounts.get(&bob()).is_none());
    let a = ex.accounts.get_mut(&alice()).unwrap();
    a.nonce = 1; // same value: no flush op
    assert!(testing::cache_slots() > 1, "non-root slots are cached");
    testing::assert_view_clean();
    assert_eq!(end_call().rwd().1, 0);
}

/// Kills `has_pending_writes -> false` and `== -> !=` (the root exclusion): a put of a new
/// record at a non-root key.
#[test]
#[should_panic(expected = "state: a view call has pending writes")]
fn gaps_view_that_inserts_fails_the_view_check() {
    let mut ex = ex_with_alice();
    let _ = ex.accounts.try_insert(bob(), Account::default());
    testing::assert_view_clean();
}

/// As above, through a pending remove of a stored key.
#[test]
#[should_panic(expected = "state: a view call has pending writes")]
fn gaps_view_that_removes_fails_the_view_check() {
    let mut ex = ex_with_alice();
    ex.accounts.at_mut(&alice()).unwrap().balances.discard(&"usdt".into());
    testing::assert_view_clean();
}

/// As above, through a `get_mut` that changes the value (touched, bytes differ).
#[test]
#[should_panic(expected = "state: a view call has pending writes")]
fn gaps_view_that_mutates_through_get_mut_fails_the_view_check() {
    let mut ex = ex_with_alice();
    ex.accounts.get_mut(&alice()).unwrap().nonce = 2;
    testing::assert_view_clean();
}

// ---------------------------------------------------------------------------------------
// 2. `StorageKey::MAX_LEN` of tuples and options
// ---------------------------------------------------------------------------------------

/// Kills `+ -> -` / `+ -> *` in the `(A, B)` and `Option<A>` impls. Operands chosen so every
/// mutant still compiles and gives a different value.
#[test]
fn gaps_compound_key_max_len_constants() {
    assert_eq!(<(u16, u8) as StorageKey>::MAX_LEN, 3);
    assert_eq!(<(u64, u32) as StorageKey>::MAX_LEN, 12);
    assert_eq!(<(String, u8) as StorageKey>::MAX_LEN, 517);
    assert_eq!(<Option<u8> as StorageKey>::MAX_LEN, 2);
    assert_eq!(<(Option<u8>, (u16, u8)) as StorageKey>::MAX_LEN, 5);
}

fn entry_of<K: StorageKey>(k: &K) -> Path {
    Path::root(DEFAULT_ROOT).field(1).kind(kind::ENTRIES).entry(k)
}

/// A tuple key of exactly `MAX_LEN` bytes is accepted.
#[test]
fn gaps_tuple_key_of_exactly_max_len_is_accepted() {
    let k = ("x".repeat(512), 7u8);
    assert_eq!(borsh::to_vec(&k).unwrap().len(), <(String, u8) as StorageKey>::MAX_LEN);
    let _ = entry_of(&k);
}

#[test]
#[should_panic(expected = "storage key is 518 bytes, over its declared MAX_LEN of 517")]
fn gaps_tuple_key_one_over_max_len_is_refused() {
    let _ = entry_of(&("x".repeat(513), 7u8));
}

/// An option key of exactly `MAX_LEN` bytes is accepted. (Only `Option<u8>`-sized operands
/// here: `Option<String>` would make the `+ -> -` mutant a const-eval underflow, a build failure
/// rather than a failing test.)
#[test]
fn gaps_option_key_of_exactly_max_len_is_accepted() {
    let k = Some(9u8);
    assert_eq!(borsh::to_vec(&k).unwrap().len(), <Option<u8> as StorageKey>::MAX_LEN);
    let _ = entry_of(&k);
    let _ = entry_of(&(Some(1u8), (2u16, 3u8)));
}

// ---------------------------------------------------------------------------------------
// 3. The derive's `MAX_SUBPATH` helpers
// ---------------------------------------------------------------------------------------

/// Kills `varint_len -> 1`, `+= -> -=`, `+= -> *=`, `>>= -> <<=`.
#[test]
fn gaps_varint_len_boundaries() {
    for (v, n) in [
        (0u32, 1usize),
        (1, 1),
        (127, 1),
        (128, 2),
        (16_383, 2),
        (16_384, 3),
        (2_097_151, 3),
        (2_097_152, 4),
        (268_435_455, 4),
        (268_435_456, 5),
        (u32::MAX, 5),
    ] {
        assert_eq!(p::varint_len(v), n, "varint_len({v})");
        let mut buf = StdVec::new();
        record::write_varint(&mut buf, v as u64);
        assert_eq!(buf.len(), n, "agrees with write_varint for {v}");
    }
}

/// Kills `&& -> ||` and `== -> !=` in `field_subpath`. Four shapes: an inline leaf (0), an
/// inline record that owns collections, a collection, and a `Lazy` of a leaf (not inline but
/// `MAX_SUBPATH == 0`: still costs its tag).
#[test]
fn gaps_field_subpath_shapes() {
    let acct = <Account as Value>::MAX_SUBPATH;
    assert!(<Account as Value>::INLINE && acct > 0);
    assert!(!<Lazy<u64> as Value>::INLINE && <Lazy<u64> as Value>::MAX_SUBPATH == 0);
    let map = <Map<u8, u8> as Value>::MAX_SUBPATH;

    assert_eq!(p::field_subpath::<u64>(1), 0);
    assert_eq!(p::field_subpath::<u64>(300), 0);
    assert_eq!(p::field_subpath::<Account>(3), 1 + acct);
    assert_eq!(p::field_subpath::<Account>(200), 2 + acct);
    assert_eq!(p::field_subpath::<Map<u8, u8>>(5), 1 + map);
    assert_eq!(p::field_subpath::<Map<u8, u8>>(16_384), 3 + map);
    assert_eq!(p::field_subpath::<Lazy<u64>>(7), 1);
    assert_eq!(p::field_subpath::<Lazy<u64>>(128), 2);
}

/// `max` with distinct arguments, both orders. (`> -> >=` is equivalent: with equal arguments
/// both branches return the same value.)
#[test]
fn gaps_max() {
    assert_eq!(p::max(3, 9), 9);
    assert_eq!(p::max(9, 3), 9);
    assert_eq!(p::max(4, 4), 4);
    assert_eq!(p::max(0, 0), 0);
}

// ---------------------------------------------------------------------------------------
// 4. `EnvHost::has_key`, `Path::is_empty`, `Path`'s `Debug`
// ---------------------------------------------------------------------------------------

/// Kills `EnvHost::has_key -> true`: `contains_key` on an unread key goes to
/// `storage_has_key` on the mocked blockchain.
#[test]
fn gaps_env_host_has_key_answers_absent_and_present() {
    let _ = crate::mock::with_mocked_blockchain(|b| b.take_storage());
    testing::use_env_host();
    testing::begin_call();
    let mut m: LookupMap<u32, u32> = at(1);
    assert!(!m.contains_key(&7));
    m.insert(7, 70);
    testing::flush();
    testing::begin_call();
    assert!(m.contains_key(&7));
    assert!(!m.contains_key(&8));
    testing::begin_call();
}

/// Kills `Path::is_empty -> true`. (`-> false` is equivalent: every `Path` constructor starts
/// from a one-byte root and only appends, so no `Path` is empty.)
#[test]
fn gaps_path_is_never_empty() {
    let r = Path::root(DEFAULT_ROOT);
    assert!(!r.is_empty());
    assert!(!r.field(1).kind(kind::ENTRIES).entry(&1u8).is_empty());
}

/// Kills the `Debug for Path` mutant: lowercase hex, two digits per byte, no separators.
#[test]
fn gaps_path_debug_is_zero_padded_hex() {
    assert_eq!(format!("{:?}", Path::root(DEFAULT_ROOT).field(3).kind(kind::ENTRIES)), "f00300");
    assert_eq!(format!("{:?}", Path::root(0x0a).field(300)), "0aac02");
    assert_eq!(format!("{:?}", Path::root(0)), "00");
}

// ---------------------------------------------------------------------------------------
// 5. `Display for CursorGone`, `Debug for Lazy`
// ---------------------------------------------------------------------------------------

#[test]
fn gaps_cursor_gone_display_is_its_message() {
    let s = CursorGone.to_string();
    assert_eq!(s, CursorGone.as_ref());
    assert_eq!(
        s,
        "state: page cursor key no longer exists (removed since the last page); restart from a \
         known key or use offset paging"
    );
}

#[test]
fn gaps_lazy_debug_shows_its_binding() {
    assert_eq!(format!("{:?}", Lazy::<u64>::default()), "Lazy(Unbound)");
    assert_eq!(format!("{:?}", Lazy::new(5u64)), "Lazy(Unbound)");
    let p = Path::root(DEFAULT_ROOT).field(1);
    assert_eq!(format!("{:?}", Lazy::<u64>::__bound_at(p.clone())), "Lazy(Bound(f001))");
    let mut l = Lazy::<u64>::__bound_at(p);
    Value::detach(&mut l);
    assert_eq!(format!("{:?}", l), "Lazy(Detached)");
}

// ---------------------------------------------------------------------------------------
// 6. `IndexIter`, `len` / `is_empty`, `At<LookupMap>`
// ---------------------------------------------------------------------------------------

/// Kills the `size_hint` mutants and `next_back` / `nth_back -> None`.
#[test]
fn gaps_index_iter_is_exact_and_double_ended() {
    testing::use_mock();
    let mut v: Vector<u32> = at(1);
    for i in 0..6 {
        v.push(10 + i);
    }
    end_call();
    let mut it = v.iter();
    assert_eq!(it.size_hint(), (6, Some(6)));
    assert_eq!(it.len(), 6);
    assert_eq!(it.next(), Some(&10));
    assert_eq!(it.size_hint(), (5, Some(5)));
    assert_eq!(it.next_back(), Some(&15));
    assert_eq!(it.size_hint(), (4, Some(4)));
    assert_eq!(it.nth_back(1), Some(&13), "skips 14");
    assert_eq!(it.size_hint(), (2, Some(2)));
    assert_eq!(it.next(), Some(&11));
    assert_eq!(it.next_back(), Some(&12));
    assert_eq!(it.size_hint(), (0, Some(0)));
    assert_eq!(it.next_back(), None);
    assert_eq!(it.next(), None);
    assert_eq!(it.nth_back(0), None);

    let mut it = v.iter();
    assert_eq!(it.nth_back(5), Some(&10));
    assert_eq!(it.size_hint(), (0, Some(0)));
    assert_eq!(v.iter().nth_back(6), None);
    assert_eq!(v.iter().rev().copied().collect::<StdVec<_>>(), vec![15, 14, 13, 12, 11, 10]);
    assert_eq!(v.iter().nth_back(0), Some(&15));
    assert_eq!(v.iter().rev().skip(2).copied().collect::<StdVec<_>>(), vec![13, 12, 11, 10]);
}

/// A map's key iterator after a removal (the last key moves into the hole), walked from both
/// ends.
#[test]
fn gaps_map_iter_after_removal_from_both_ends() {
    testing::use_mock();
    let mut m: Map<u32, u32> = at(2);
    for k in 0..5 {
        let _ = m.insert(k, k * 10);
    }
    end_call();
    assert!(m.discard(&1));
    end_call();
    assert_eq!(m.keys().collect::<StdVec<_>>(), vec![0, 4, 2, 3]);
    let mut it = m.iter();
    assert_eq!(it.size_hint(), (4, Some(4)));
    assert_eq!(it.next_back().map(|(k, v)| (k, *v)), Some((3, 30)));
    assert_eq!(it.next().map(|(k, v)| (k, *v)), Some((0, 0)));
    assert_eq!(it.size_hint(), (2, Some(2)));
    assert_eq!(it.nth_back(0).map(|(k, v)| (k, *v)), Some((2, 20)));
    assert_eq!(it.next_back().map(|(k, v)| (k, *v)), Some((4, 40)));
    assert!(it.next().is_none());
    assert_eq!(m.keys().rev().collect::<StdVec<_>>(), vec![3, 2, 4, 0]);
}

/// Kills the `len` / `is_empty` mutants of `Map`, `Set` and `Vector`, including after
/// everything is removed (a map keeps a `len` 0 header after a removal).
#[test]
fn gaps_len_and_is_empty_of_map_set_vector() {
    testing::use_mock();
    let mut m: Map<u32, u32> = at(1);
    let mut s: Set<u32> = at(2);
    let mut v: Vector<u32> = at(3);
    assert_eq!((m.len(), m.is_empty()), (0, true));
    assert_eq!((s.len(), s.is_empty()), (0, true));
    assert_eq!((v.len(), v.is_empty()), (0, true));

    for k in 0..3 {
        let _ = m.insert(k, k);
        s.insert(k);
        v.push(k);
    }
    assert_eq!((m.len(), m.is_empty()), (3, false));
    assert_eq!((s.len(), s.is_empty()), (3, false));
    assert_eq!((v.len(), v.is_empty()), (3, false));
    end_call();
    assert_eq!((m.len(), m.is_empty()), (3, false));
    assert_eq!((s.len(), s.is_empty()), (3, false));
    assert_eq!((v.len(), v.is_empty()), (3, false));

    assert!(m.discard(&0));
    assert!(s.remove(&0));
    assert_eq!(v.pop(), Some(2));
    end_call();
    assert_eq!((m.len(), m.is_empty()), (2, false));
    assert_eq!((s.len(), s.is_empty()), (2, false));
    assert_eq!((v.len(), v.is_empty()), (2, false));

    for k in 1..3 {
        assert!(m.discard(&k));
        assert!(s.remove(&k));
    }
    while v.pop().is_some() {}
    assert_eq!((m.len(), m.is_empty()), (0, true));
    assert_eq!((s.len(), s.is_empty()), (0, true));
    assert_eq!((v.len(), v.is_empty()), (0, true));
    end_call();
    assert_eq!((m.len(), m.is_empty()), (0, true));
    assert_eq!((s.len(), s.is_empty()), (0, true));
    assert_eq!((v.len(), v.is_empty()), (0, true));
    assert!(
        testing::store()
            .keys()
            .any(|k| k.starts_with(Path::root(DEFAULT_ROOT).field(1).as_bytes())),
        "the emptied map keeps its header"
    );
}

/// Kills `At<LookupMap>::get -> None` and `contains_key -> true / false`: the read-only
/// handle a record view gives for a `LookupMap` field.
#[test]
fn gaps_at_lookup_map_get_and_contains_key() {
    testing::use_mock();
    let field = Path::root(DEFAULT_ROOT).field(4);
    let mut m: LookupMap<u8, u32> = at(4);
    m.insert(1, 100);
    m.insert(2, 200);
    end_call();
    m.discard(&2);
    end_call();
    let view: At<'_, LookupMap<u8, u32>> =
        <LookupMap<u8, u32> as ViewField<'_, u64>>::__view_field(field);
    assert_eq!(view.get(&1), Some(&100));
    assert_eq!(view.get(&2), None);
    assert_eq!(view.get(&3), None);
    assert!(view.contains_key(&1));
    assert!(!view.contains_key(&2));
    assert!(!view.contains_key(&3));
    testing::assert_view_clean();
}

// ---------------------------------------------------------------------------------------
// 7. `MAX_SUBPATH` of the collections
// ---------------------------------------------------------------------------------------

/// Kills the `+ -> - / *` mutants of the map-like and `Vector` `MAX_SUBPATH` constants.
/// Operands avoid 0 and 1 (and stay small enough that every mutant still compiles) so each
/// mutant gives a different value: `Vector<u8>` (5) as an inner value, `u32` (4) as a key.
#[test]
fn gaps_collection_max_subpath_constants() {
    let h = ENTRY_HASH_LEN;
    assert_eq!(<Vector<u8> as Value>::MAX_SUBPATH, 5);
    assert_eq!(<Vector<Vector<u8>> as Value>::MAX_SUBPATH, 1 + 4 + 5);
    assert_eq!(<LookupMap<u32, u8> as Value>::MAX_SUBPATH, 1 + h + 4);
    assert_eq!(<LookupMap<u32, Vector<u8>> as Value>::MAX_SUBPATH, 1 + h + 4 + 5);
    assert_eq!(<Map<u32, u8> as Value>::MAX_SUBPATH, 1 + h + 4);
    assert_eq!(<Map<u32, Vector<u8>> as Value>::MAX_SUBPATH, 1 + h + 4 + 5);
    assert_eq!(<Set<u32> as Value>::MAX_SUBPATH, 1 + h + 4);
    assert_eq!(<LookupSet<u32> as Value>::MAX_SUBPATH, 1 + h + 4);
    assert_eq!(<Lazy<Vector<u8>> as Value>::MAX_SUBPATH, 5);
    assert_eq!(<GroupedMap<u32, u8, u8> as Value>::MAX_SUBPATH, 1 + h + 4 + (1 + h + 1));
    assert_eq!(<GroupedMap<u64, u8, u8> as Value>::MAX_SUBPATH, 1 + h + 8 + (1 + h + 1));
}

// ---------------------------------------------------------------------------------------
// 8. `assert_insert_new`: `Lazy::set` of a record that owns collections
// ---------------------------------------------------------------------------------------

/// `set` on an empty `Lazy<Account>` writes; a leaf `Lazy` may be overwritten freely.
#[test]
fn gaps_lazy_set_of_a_record_over_empty_works() {
    testing::use_mock();
    let mut l: Lazy<Account> = at(5);
    assert!(l.get().is_none());
    l.set(Account { nonce: 3, ..Default::default() });
    end_call();
    assert_eq!(l.get().map(|a| a.nonce), Some(3));
    let mut n: Lazy<u64> = at(6);
    n.set(1);
    end_call();
    n.set(2);
    end_call();
    assert_eq!(n.get(), Some(&2));
}

/// Kills `assert_insert_new -> ()` and `> -> <`: overwriting a stored record (and so
/// orphaning its collections) is refused.
#[test]
#[should_panic(expected = "state: record exists; use get_mut, or remove it first")]
fn gaps_lazy_set_of_a_record_over_a_stored_one_panics() {
    testing::use_mock();
    let mut l: Lazy<Account> = at(5);
    l.set(Account::default());
    end_call();
    l.set(Account { nonce: 9, ..Default::default() });
}

/// As above, within one call (the cache sees the value set earlier in the call).
#[test]
#[should_panic(expected = "state: record exists; use get_mut, or remove it first")]
fn gaps_lazy_set_of_a_record_twice_in_one_call_panics() {
    testing::use_mock();
    let mut l: Lazy<Account> = at(5);
    l.set(Account::default());
    l.set(Account::default());
}

/// `Lazy::new` bound over a stored record: `bind` drains the pending value through `set`, so
/// the same guard refuses it. (The `__write_pending` call site is not driven here.)
#[test]
#[should_panic(expected = "state: record exists; use get_mut, or remove it first")]
fn gaps_lazy_new_bound_over_a_stored_record_panics() {
    testing::use_mock();
    let mut l: Lazy<Account> = at(5);
    l.set(Account::default());
    end_call();
    let mut fresh = Lazy::new(Account::default());
    fresh.bind(&Path::root(DEFAULT_ROOT).field(5));
}

// ---------------------------------------------------------------------------------------
// Error litmus (verification round 2): error paths no test pinned
// ---------------------------------------------------------------------------------------

/// A `#[key(n, legacy)]` root field missing from the stored root is `Missing { tag }`, whose
/// message names the tag (litmus: replacing it with another error survived every suite).
#[test]
fn gaps_legacy_field_missing_is_missing_naming_the_tag() {
    use super::record::{RecordError, Wire};
    let other = 7u64.to_le_bytes();
    let fields: p::Fields<'_> = vec![(1, Wire::Scalar, &other[..])];
    let e = p::legacy_field::<u64>(&fields, 2).expect_err("tag 2 is absent");
    assert_eq!(e, RecordError::Missing { tag: 2 });
    assert!(e.to_string().contains("required field tag 2 is absent"), "{e}");
    // Present: decoded as its Borsh bytes.
    assert_eq!(p::legacy_field::<u64>(&fields, 1), Ok(7));
}

/// `#[key(n, required)]` on a collection is refused at decode, present or absent (litmus:
/// deleting the check survived every suite).
#[test]
fn gaps_required_on_a_collection_field_is_refused() {
    use super::record::{RecordError, Wire};
    let path = Path::root(DEFAULT_ROOT);
    let marker = [0x81u8];
    let present: p::Fields<'_> = vec![(3, Wire::Marker, &marker[..])];
    for fields in [vec![], present] {
        for path in [None, Some(&path)] {
            let e = p::field_required::<LookupMap<u8, u8>>(&fields, 3, path)
                .map(|_| ())
                .expect_err("required on a collection");
            assert_eq!(e, RecordError::Leaf("`required` on a collection field".into()));
        }
    }
    // The inline case still decodes.
    let v = 5u32.to_le_bytes();
    let f: p::Fields<'_> = vec![(3, Wire::Scalar, &v[..])];
    assert_eq!(p::field_required::<u32>(&f, 3, Some(&path)), Ok(5));
    assert_eq!(p::field_required::<u32>(&f, 4, Some(&path)), Err(RecordError::Missing { tag: 4 }));
}
