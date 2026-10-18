//! Tests closing cargo-mutants gaps listed in PROTOTYPE_NOTES.md ("Mutation testing", soundness
//! round of Oct 17). Each test names the mutants it kills.

use super::key::{DEFAULT_ROOT, Path, kind};
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
