//! Tests closing the cargo-mutants gaps of verification round 2 outside `cache.rs`
//! (survivors of the lib tests and the `state_v2_verify` / `state_v2_migrate` /
//! `state_v2_exchange` suites). Each test names the mutants it kills.

use super::key::{DEFAULT_ROOT, Path};
use super::record::{self, Wire, marker};
use super::root::__private as p;
use super::testing::{self, end_call, seed, store};
use super::tests::Account;
use super::verify::fixture::RecX;
use super::*;
use std::vec::Vec as StdVec;

fn at<C: Default + Value>(tag: u32) -> C {
    let mut c = C::default();
    c.bind(&Path::root(DEFAULT_ROOT).field(tag));
    c
}

fn field(tag: u32) -> Path {
    Path::root(DEFAULT_ROOT).field(tag)
}

/// Runs `f`, which must panic, and returns its message.
fn panic_msg(f: impl FnOnce()) -> String {
    let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).expect_err("must panic");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .expect("string panic payload")
}

// ---------------------------------------------------------------------------------------
// 1. `At` views of `Map`, `Set` and `Vector`
// ---------------------------------------------------------------------------------------

/// Kills `At<Map>::contains_key -> true / false`, `is_empty -> true / false`,
/// `key_at -> None` and `revision -> 0 / 1`: the read-only handle a record view gives for a
/// `Map` field, empty and after two removals (revision 2).
#[test]
fn gaps_rest_at_map_reads() {
    testing::use_mock();
    let view = |tag| -> At<'static, Map<u8, u32>> {
        <Map<u8, u32> as ViewField<'static, u64>>::__view_field(field(tag))
    };
    let empty = view(9);
    assert!(empty.is_empty());
    assert_eq!(empty.revision(), 0);
    assert_eq!(empty.key_at(0), None);
    assert!(!empty.contains_key(&0));

    let mut m: Map<u8, u32> = at(1);
    for k in 0..4 {
        let _ = m.insert(k, u32::from(k) * 10);
    }
    end_call();
    assert!(m.discard(&1));
    assert!(m.discard(&2));
    end_call();
    // Keys 0, 1, 2, 3; removing 1 moves 3 into slot 1; removing 2 (last) moves nothing.
    let v = view(1);
    assert!(!v.is_empty());
    assert_eq!(v.len(), 2);
    assert_eq!(v.revision(), 2);
    assert_eq!(v.key_at(0), Some(0));
    assert_eq!(v.key_at(1), Some(3));
    assert_eq!(v.key_at(2), None);
    assert!(v.contains_key(&0));
    assert!(v.contains_key(&3));
    assert!(!v.contains_key(&1));
    assert!(!v.contains_key(&7));
    testing::assert_view_clean();
}

/// Kills `At<Set>::is_empty -> true / false` and `key_at -> None`.
#[test]
fn gaps_rest_at_set_reads() {
    testing::use_mock();
    let view = |tag| -> At<'static, Set<u8>> {
        <Set<u8> as ViewField<'static, u64>>::__view_field(field(tag))
    };
    let empty = view(9);
    assert!(empty.is_empty());
    assert_eq!(empty.key_at(0), None);

    let mut s: Set<u8> = at(2);
    s.insert(5);
    s.insert(6);
    end_call();
    let v = view(2);
    assert!(!v.is_empty());
    assert_eq!(v.len(), 2);
    assert_eq!(v.key_at(0), Some(5));
    assert_eq!(v.key_at(1), Some(6));
    assert_eq!(v.key_at(2), None);
    assert!(v.contains(&6));
    testing::assert_view_clean();
}

/// Kills `At<Vector>::is_empty -> true / false`.
#[test]
fn gaps_rest_at_vector_is_empty() {
    testing::use_mock();
    let view = |tag| -> At<'static, Vector<u32>> {
        <Vector<u32> as ViewField<'static, u64>>::__view_field(field(tag))
    };
    assert!(view(9).is_empty());
    let mut v: Vector<u32> = at(3);
    v.push(1);
    end_call();
    let w = view(3);
    assert!(!w.is_empty());
    assert_eq!(w.len(), 1);
    testing::assert_view_clean();
}

// ---------------------------------------------------------------------------------------
// 2. `Value` impls of `Lazy`, `Set` and `LookupSet`
// ---------------------------------------------------------------------------------------

/// Kills `Lazy::path -> None` and the deleted `Binding::Bound` arm.
#[test]
fn gaps_rest_lazy_path_is_its_binding() {
    assert_eq!(Lazy::<u64>::default().path(), None);
    assert_eq!(Lazy::new(1u64).path(), None);
    let p = field(4);
    let mut l = Lazy::<u64>::__bound_at(p.clone());
    assert_eq!(l.path(), Some(&p));
    let mut b: Lazy<u64> = Lazy::default();
    b.bind(&p);
    assert_eq!(b.path(), Some(&p));
    Value::detach(&mut l);
    assert_eq!(l.path(), None);
}

/// Kills `<impl Value for Lazy<T>>::encode_value -> vec![] / vec![0] / vec![1]`: a collection
/// has no inline encoding, so encoding one fails closed instead of writing bytes. (No SDK path
/// reaches it since `Lazy<T>` refuses a collection `T` at compile time; this pins the backstop
/// behind that check.)
#[test]
fn gaps_rest_lazy_refuses_inline_encoding() {
    let m = panic_msg(|| {
        let _ = Value::encode_value(&Lazy::<u64>::default(), None);
    });
    assert!(m.contains("collections are not stored inline"), "{m}");
}

/// Kills `<impl Value for Set<K>>::encode_value -> vec![] / vec![0] / vec![1]`, as above.
#[test]
fn gaps_rest_set_refuses_inline_encoding() {
    let m = panic_msg(|| {
        let _ = Value::encode_value(&Set::<u8>::default(), Some(&[1]));
    });
    assert!(m.contains("unreachable"), "{m}");
}

/// Kills `<impl Value for LookupSet<K>>::encode_value -> vec![] / vec![0] / vec![1]`, as above.
#[test]
fn gaps_rest_lookup_set_refuses_inline_encoding() {
    let m = panic_msg(|| {
        let _ = Value::encode_value(&LookupSet::<u8>::default(), None);
    });
    assert!(m.contains("unreachable"), "{m}");
}

/// Kills the `decode_value -> Ok(Default::default())` mutants of `Lazy`, `Set` and
/// `LookupSet`: decoding bytes as a collection is an error (the backstop, as above).
#[test]
fn gaps_rest_collections_refuse_inline_decoding() {
    let p = field(1);
    let want = RecordError::Leaf("a collection has no inline encoding".into());
    for bytes in [&[][..], &[0x81][..], &[1, 2, 3][..]] {
        assert_eq!(<Lazy<u64> as Value>::decode_value(bytes, &p).map(|_| ()), Err(want.clone()));
        assert_eq!(<Set<u8> as Value>::decode_value(bytes, &p).map(|_| ()), Err(want.clone()));
        assert_eq!(
            <LookupSet<u8> as Value>::decode_value(bytes, &p).map(|_| ()),
            Err(want.clone())
        );
    }
}

/// Kills `<impl Value for Set<K>>::detach -> ()`: a record removed from a map comes back with
/// its `Set` detached, so touching it panics instead of reading the cleared keys.
#[test]
fn gaps_rest_removed_record_set_is_detached() {
    testing::use_mock();
    let mut m: Map<u8, RecX> = at(6);
    let r = m.try_insert(1, RecX::default()).ok().unwrap();
    r.n = 3;
    r.set.insert(9);
    end_call();
    let rx = m.remove(&1).expect("present");
    assert_eq!(rx.n, 3);
    let msg = panic_msg(|| {
        let _ = rx.set.contains(&9);
    });
    assert!(msg.contains("collection detached by remove"), "{msg}");
    let msg = panic_msg(|| {
        let _ = rx.set.len();
    });
    assert!(msg.contains("collection detached by remove"), "{msg}");
}

/// Kills `<impl Value for LookupSet<K>>::detach -> ()`. No SDK removal reaches it (a record
/// holding a `LookupSet` is not `Removable`), so this pins the `Value::detach` contract
/// directly: after `detach`, the set panics on use.
#[test]
fn gaps_rest_lookup_set_detach_detaches() {
    testing::use_mock();
    let mut s: LookupSet<u8> = at(3);
    s.insert(1);
    end_call();
    assert!(s.contains(&1));
    Value::detach(&mut s);
    let msg = panic_msg(|| {
        let _ = s.contains(&1);
    });
    assert!(msg.contains("collection detached by remove"), "{msg}");
}

/// Kills `<impl Value for Lazy<T>>::__write_pending -> ()`: a `Lazy::new` in a root built as
/// a struct literal is written by the flush's `__write_pending` walk (1 write), once.
#[test]
fn gaps_rest_lazy_write_pending_writes_once() {
    testing::use_mock();
    let p = field(4);
    let l = Lazy::new(7u64);
    Value::__write_pending(&l, &p);
    Value::__write_pending(&l, &p); // the pending value was taken: nothing more
    let r = end_call();
    assert_eq!(r.rwd(), (0, 1, 0));
    assert_eq!(store().get(p.as_bytes()), Some(&7u64.to_le_bytes().to_vec()));
    // An empty `Lazy` writes nothing.
    Value::__write_pending(&Lazy::<u64>::default(), &field(5));
    assert_eq!(end_call().rwd(), (0, 0, 0));
}

// ---------------------------------------------------------------------------------------
// 3. Record markers and the unknown-collection message
// ---------------------------------------------------------------------------------------

/// Kills `marker::is_marker -> true` and `&& -> ||`.
#[test]
fn gaps_rest_is_marker_is_exactly_the_defined_kinds() {
    for b in 0..=u8::MAX {
        assert_eq!(marker::is_marker(b), (0x81..=0x87).contains(&b), "{b:#x}");
    }
}

/// Kills the six deleted arms of `marker::name` (the `MAP` arm is pinned elsewhere).
#[test]
fn gaps_rest_marker_names() {
    for (k, n) in [
        (marker::MAP, "Map"),
        (marker::LOOKUP_MAP, "LookupMap"),
        (marker::SET, "Set"),
        (marker::LOOKUP_SET, "LookupSet"),
        (marker::VECTOR, "Vector"),
        (marker::LAZY, "Lazy"),
        (marker::GROUPED_MAP, "GroupedMap"),
        (0x88, "unknown collection"),
        (0x00, "unknown collection"),
    ] {
        assert_eq!(marker::name(k), n, "{k:#x}");
    }
}

/// Stored `Account` bytes: `nonce` 1, plus `extra` (already encoded fields, tags above 2).
fn account_bytes_with(extra: &[u8]) -> StdVec<u8> {
    let mut b = vec![record::field_key(1, Wire::Scalar) as u8, 8];
    b.extend_from_slice(&1u64.to_le_bytes());
    b.extend_from_slice(extra);
    b
}

/// The removal check's panic names the kind the unknown tag's marker declares (through
/// `marker::name`), and for a top-level tag suggests `reserved(t)` by number. Kills the
/// deleted `[t]` arm of `unknown_collection_panic` and, through the message, the deleted arms
/// of `marker::name`.
#[test]
fn gaps_rest_unknown_collection_panic_names_kind_and_tag() {
    let marker_key = record::field_key(7, Wire::Marker) as u8;
    for (k, n) in [
        (marker::MAP, "Map"),
        (marker::LOOKUP_MAP, "LookupMap"),
        (marker::SET, "Set"),
        (marker::LOOKUP_SET, "LookupSet"),
        (marker::VECTOR, "Vector"),
        (marker::LAZY, "Lazy"),
        (marker::GROUPED_MAP, "GroupedMap"),
        (0x90, "unknown collection"),
    ] {
        testing::use_mock();
        seed(field(5).as_bytes(), &account_bytes_with(&[marker_key, 1, k]));
        let mut l: Lazy<Account> = at(5);
        let msg = panic_msg(|| l.remove());
        assert!(msg.contains(&format!("carry tag 7, a `{n}` field")), "{msg}");
        assert!(msg.contains("list the tag in `#[near(storage, reserved(7))]`"), "{msg}");
    }
    // A nested trail (tag 7 is a nested record holding a `Set` marker at its tag 2) names the
    // path instead.
    testing::use_mock();
    let inner = [record::field_key(2, Wire::Marker) as u8, 1, marker::SET];
    let mut extra = vec![record::field_key(7, Wire::Record) as u8, inner.len() as u8];
    extra.extend_from_slice(&inner);
    seed(field(5).as_bytes(), &account_bytes_with(&extra));
    let mut l: Lazy<Account> = at(5);
    let msg = panic_msg(|| l.remove());
    assert!(msg.contains("carry tag 7 > 2, a `Set` field"), "{msg}");
    assert!(msg.contains("`reserved(n)` on the record that dropped it (tag path 7 > 2)"), "{msg}");
}

/// Kills the six `Value::__unknown_collection -> Some(..)` mutants of the default body: a
/// declared leaf or collection field whose stored bytes are wire 2 (a field once declared as
/// a nested record, re-typed by a later version) holds no collection, so removal proceeds.
#[test]
fn gaps_rest_default_unknown_collection_is_none() {
    assert_eq!(<u64 as Value>::__unknown_collection(&[]), None);
    assert_eq!(<u64 as Value>::__unknown_collection(&[0x09, 1, 0x81]), None);
    assert_eq!(<Map<u8, u8> as Value>::__unknown_collection(&[0x09, 1, 0x81]), None);
    assert_eq!(p::declared_field_collection::<u64>(Wire::Record, &[0x09, 1, 0x81]), None);

    // Through a removal: `nonce` (tag 1, declared `u64`) stored as wire 2, 8 bytes.
    testing::use_mock();
    let mut b = vec![record::field_key(1, Wire::Record) as u8, 8];
    b.extend_from_slice(&[0x09, 1, 0x81, 0, 0, 0, 0, 0]);
    seed(field(5).as_bytes(), &b);
    let mut l: Lazy<Account> = at(5);
    l.remove();
    end_call();
    assert!(!store().contains_key(field(5).as_bytes()), "the record is removed");
}

// ---------------------------------------------------------------------------------------
// 4. Removal: `Removal::is_done`, `Occupied`, `guard_removal`'s flag-key decision
// ---------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------
// A hand-expanded leaf-only record: #[near(storage)] struct Pt { #[key(1)] x: u64 }
// (`__RECORD`, but `MAX_SUBPATH == 0`; no fixture has this shape)
// ---------------------------------------------------------------------------------------

#[derive(Default, Debug, Clone, PartialEq)]
struct Pt {
    x: u64,
}

impl Value for Pt {
    const MAX_SUBPATH: usize = p::field_subpath::<u64>(1);
    const __RECORD: bool = true;
    fn __unknown_collection(bytes: &[u8]) -> Option<(StdVec<u32>, u8)> {
        let f = p::parse(bytes).ok()?;
        for &(t, w, b) in f.iter() {
            let hit = match t {
                1 => p::declared_field_collection::<u64>(w, b),
                _ => p::unknown_field_collection(w, b),
            };
            if let Some((mut trail, k)) = hit {
                trail.insert(0, t);
                return Some((trail, k));
            }
        }
        None
    }
    fn encode_value(&self, orig: Option<&[u8]>) -> StdVec<u8> {
        let of = p::parse_orig(orig);
        let mut w = p::RecordWriter::new();
        p::encode_field(&mut w, of.as_ref(), 1, &self.x);
        w.finish(orig, |t| match t {
            1 => !<u64 as Value>::INLINE,
            _ => true,
        })
    }
    fn decode_value(bytes: &[u8], path: &Path) -> Result<Self, RecordError> {
        let f = p::parse(bytes)?;
        Ok(Self { x: p::field(&f, 1, Some(path))? })
    }
}
impl Leaf for Pt {}
impl Removable for Pt {}

/// Kills `Removal::is_done -> true / false` and `== -> !=`.
#[test]
fn gaps_rest_removal_is_done() {
    assert!(Removal::Done.is_done());
    assert!(!Removal::Pending.is_done());
}

/// Kills the `Display for Occupied` and `AsRef<str> for Occupied` mutants.
#[test]
fn gaps_rest_occupied_display_is_its_message() {
    let s = Occupied.to_string();
    assert_eq!(s, Occupied.as_ref());
    assert_eq!(s, "state: key already present; nothing was inserted");
}

/// Kills `guard_removal`'s `&& -> ||` and `> -> >=`: the removal check reads a key's D9 flag
/// key on a miss only for a map entry (`entry`) of a value that owns storage. A leaf-only
/// record (`MAX_SUBPATH == 0`) discarded from a `LookupMap`, and a record that owns storage
/// removed from a `Lazy` (not an entry), both read just the key.
#[test]
fn gaps_rest_guard_removal_reads_the_flag_key_only_for_flaggable_entries() {
    testing::use_mock();
    let mut lm: LookupMap<u8, Pt> = at(1);
    lm.discard(&9);
    let r = end_call();
    assert_eq!((r.rwd(), r.has_keys()), ((1, 0, 0), 0), "{r:?}");

    let mut l: Lazy<Account> = at(5);
    l.remove();
    let r = end_call();
    assert_eq!((r.rwd(), r.has_keys()), ((2, 0, 0), 0), "{r:?}");

    // Stored, the leaf-only record is read once and removed once.
    lm.insert(9, Pt { x: 4 });
    end_call();
    lm.discard(&9);
    let r = end_call();
    assert_eq!((r.rwd(), r.has_keys()), ((1, 0, 1), 0), "{r:?}");
    assert!(store().is_empty());
}
// ---------------------------------------------------------------------------------------
// 5. Migrating: `NewLookupMap::target_try_insert`
// ---------------------------------------------------------------------------------------

/// A v1 map for the tests: `(k, v)` pairs, stored as Borsh, prefix `x`.
impl LegacyMap for StdVec<(u8, u32)> {
    type K = u8;
    type V = u32;
    fn legacy_get(&self, k: &u8) -> Option<u32> {
        self.iter().find(|(x, _)| x == k).map(|(_, v)| *v)
    }
    fn legacy_remove(&mut self, k: &u8) {
        self.retain(|(x, _)| x != k);
    }
    fn legacy_prefixes(&self) -> StdVec<StdVec<u8>> {
        vec![b"x".to_vec()]
    }
}
impl LegacyIterable for StdVec<(u8, u32)> {
    fn legacy_take(&mut self, n: u32) -> StdVec<(u8, u32)> {
        let n = (n as usize).min(self.len());
        self.drain(..n).collect()
    }
    fn legacy_is_empty(&self) -> bool {
        self.is_empty()
    }
}

/// Kills `<impl MigrationTarget for NewLookupMap>::target_try_insert -> true / false`: it
/// inserts an absent key (`true`) and leaves a present one (`false`), so `migrate_batch` into
/// a v2 `LookupMap` moves new keys and never overwrites a newer value. The `Migrating` is
/// bound at the same path as the first map on purpose: key 1 is already on the new side.
#[test]
fn gaps_rest_lookup_map_target_try_insert() {
    testing::use_mock();
    let mut m: LookupMap<u8, u32> = at(1);
    assert!(m.target_try_insert(1, 10));
    assert!(!m.target_try_insert(1, 20));
    assert_eq!(m.get(&1), Some(&10));
    end_call();
    assert_eq!(m.get(&1), Some(&10));

    let mut mig: Migrating<StdVec<(u8, u32)>, LookupMap<u8, u32>> =
        Migrating::new(vec![(2, 20), (3, 30), (1, 99)]);
    Value::bind(&mut mig, &field(1));
    assert_eq!(mig.migrate_batch(5), 3);
    assert!(mig.old_is_empty());
    assert_eq!(mig.new_side().get(&2), Some(&20));
    assert_eq!(mig.new_side().get(&3), Some(&30));
    assert_eq!(mig.new_side().get(&1), Some(&10), "the newer value stays");
    end_call();
    assert_eq!(mig.get(&2), Some(20));
    assert_eq!(mig.get(&3), Some(30));
}
