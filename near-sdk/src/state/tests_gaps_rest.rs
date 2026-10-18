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
