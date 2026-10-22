//! `#[key(n, default = expr)]` and `#[key(n, default_fn = path)]` resolve in the user's scope:
//! a function named `f` or `path` (the derive's decode locals) or a const named like the one
//! the derive used to generate (`__KEY_DEFAULT`) is the user's item, not a generated binding.
#![cfg(feature = "unstable")]

use near_sdk::near;
use near_sdk::state::Value;

const fn f() -> u8 {
    3
}

const fn path() -> u8 {
    4
}

const __KEY_DEFAULT: u8 = 5;

mod defaults {
    pub const fn seven() -> u8 {
        7
    }
}

#[near(storage)]
pub struct Shadowed {
    #[key(1, default_fn = f)]
    a: u8,
    #[key(2, default_fn = path)]
    b: u8,
    #[key(3, default = f())]
    c: u8,
    #[key(4, default = path())]
    d: u8,
    #[key(5, default = __KEY_DEFAULT)]
    e: u8,
    #[key(6, default = defaults::seven())]
    g: u8,
    #[key(7, default_fn = defaults::seven)]
    h: u8,
}

#[test]
fn defaults_resolve_to_user_items() {
    let v = Shadowed::default();
    assert_eq!((v.a, v.b, v.c, v.d, v.e, v.g, v.h), (3, 4, 3, 4, 5, 7, 7));
    // a stored record without the tags decodes to the same defaults
    let path = near_sdk::state::Path::root(0xF1);
    let v = Shadowed::decode_value(&[], &path).unwrap();
    assert_eq!((v.a, v.b, v.c, v.d, v.e, v.g, v.h), (3, 4, 3, 4, 5, 7, 7));
}

/// A user `const` named like a binding of the derive's removal check. `__unknown_collection`
/// used to bind the kind byte of a hit as `k` in `if let Some((mut trail, k))`: with this const
/// in scope that was a constant pattern, so a `Map` marker (0x81 != 7) under an undeclared tag
/// was not reported and the record's removal went ahead, leaving its children behind.
#[allow(non_upper_case_globals)]
mod removal_guard {
    use near_sdk::near;
    use near_sdk::state::Map;

    pub const k: u8 = 7;

    /// The rollback target: does not declare tag 2.
    #[near(storage)]
    #[derive(Default)]
    pub struct Old {
        #[key(1, default = k)]
        pub n: u8,
    }

    /// The version that wrote the record: a child map at tag 2.
    #[near(storage)]
    #[derive(Default)]
    pub struct New {
        #[key(1)]
        pub n: u8,
        #[key(2)]
        pub kids: Map<u8, u8>,
    }
}

#[test]
fn removal_guard_ignores_user_consts() {
    use near_sdk::state::{Map, Path, testing};
    use removal_guard::{New, Old};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    // tag 2, wire 1 (a marker), `Map`'s kind byte
    assert_eq!(Old::__unknown_collection(&[9, 1, 0x81]), Some((vec![2], 0x81)));

    testing::use_mock();
    let root = Path::root(0xF0).field(9);
    let mut new: Map<u8, New> = Map::__bound_at(root.clone());
    new.try_insert(1, New::default()).unwrap();
    new.at_mut(&1).unwrap().kids.insert(5, 50);
    testing::flush();
    testing::begin_call();
    let before = testing::store();
    let removed = catch_unwind(AssertUnwindSafe(|| {
        drop(Map::<u8, Old>::__bound_at(root.clone()).remove(&1));
        testing::flush();
    }));
    assert!(removed.is_err(), "the removal of a record with an unknown `Map` tag must be refused");
    testing::begin_call();
    assert_eq!(testing::store(), before, "the refused removal changed storage");
}

/// Items named like the derive's bindings (its parameters, `let`s, closure and loop patterns):
/// each used to capture the binding as a constant pattern (E0308 / E0530). The bindings are
/// now `__near_*` and mixed-site.
#[allow(non_upper_case_globals, non_camel_case_types, dead_code)]
mod binding_names {
    use near_sdk::near;
    use near_sdk::state::{Lazy, Map};

    pub const f: u8 = 1;
    pub const path: u8 = 2;
    pub const m: u8 = 3;
    pub const of: u8 = 4;
    pub const w: u8 = 5;
    pub const orig: u8 = 6;
    pub const bytes: u8 = 7;
    pub const t: u8 = 8;
    pub const b: u8 = 9;
    pub const hit: u8 = 10;
    pub const k: u8 = 11;
    pub const v: u8 = 12;
    pub const e: u8 = 13;
    pub const anchor: u8 = 14;
    pub const field_path: u8 = 15;
    pub const budget: u8 = 16;
    /// a unit struct is a value-namespace pattern too
    pub struct trail;
    fn seventeen() -> u8 {
        17
    }
    pub const __key_default_fn: fn() -> u8 = seventeen;

    #[near(storage)]
    pub struct Inner {
        #[key(1, default = w)]
        pub x: u8,
        #[key(2)]
        pub kids: Map<u8, u8>,
    }

    #[near(storage)]
    pub struct Rec {
        #[key(1, default = f)]
        pub a: u8,
        #[key(2, default_fn = __key_default_fn)]
        pub c: u8,
        #[key(3)]
        pub inner: Inner,
        #[key(4)]
        pub lazy: Lazy<u8>,
    }

    #[near(contract_state(layout = v2, root = 0xF4))]
    pub struct Root {
        #[key(1, default = path)]
        pub a: u8,
        #[key(2)]
        pub recs: Map<u8, Rec>,
    }
}

/// Consts named like the derive's bindings, used as array lengths in field types (record and
/// legacy root fields): a field type used to resolve to a generated local (E0435) or capture it.
#[allow(non_upper_case_globals, dead_code)]
mod array_lengths {
    use near_sdk::near;
    use near_sdk::state::Map;

    pub const f: usize = 1;
    pub const path: usize = 2;
    pub const m: usize = 3;
    pub const w: usize = 4;
    pub const t: usize = 5;
    pub const b: usize = 6;
    pub const anchor: usize = 7;
    pub const budget: usize = 8;

    #[near(storage)]
    pub struct Rec {
        #[key(1, default = [7; m])]
        pub a: [u8; m],
        #[key(2)]
        pub b: [u8; f],
        #[key(3)]
        pub c: [u8; path],
        #[key(4)]
        pub d: [u8; w],
        #[key(5)]
        pub e: [u8; t],
        #[key(6)]
        pub g: [u8; b],
        #[key(7)]
        pub h: [u8; anchor],
        #[key(8)]
        pub i: [u8; budget],
        #[key(9)]
        pub kids: Map<u8, [u8; m]>,
    }

    #[near(contract_state(layout = v2, root = 0xF5))]
    pub struct Root {
        #[key(1, legacy)]
        pub l: [u8; f],
        #[key(2)]
        pub p: [u8; path],
        #[key(3)]
        pub recs: Map<u8, Rec>,
    }
}

/// `root = KEY` with a user `const KEY`: the generated `state_key` used to define its own
/// `const KEY`, which the root expression then resolved to.
mod root_const {
    use near_sdk::near;

    pub const KEY: u8 = 0xF6;

    #[near(contract_state(layout = v2, root = KEY))]
    pub struct Root {
        #[key(1)]
        pub n: u8,
    }
}

/// A field named like the views' phantom field, a record named like the `ViewField` impls'
/// parent parameter.
#[allow(non_camel_case_types)]
mod member_names {
    use near_sdk::near;
    use near_sdk::state::Map;

    #[near(storage)]
    #[derive(Default)]
    pub struct __R {
        #[key(1)]
        pub __m: u8,
        #[key(2)]
        pub kids: Map<u8, u8>,
    }

    /// `__R` nested, so its `ViewField<'a, Holder>` impl is used
    #[near(storage)]
    #[derive(Default)]
    pub struct Holder {
        #[key(1)]
        pub r: __R,
    }

    #[near(contract_state(layout = v2, root = 0xF7))]
    pub struct Root {
        #[key(1)]
        pub recs: Map<u8, __R>,
        #[key(2)]
        pub holders: Map<u8, Holder>,
    }
}

#[test]
fn user_items_named_like_generated_bindings() {
    use near_sdk::state::{ContractState, Map, Path, Root, Value, testing};

    testing::use_mock();
    // binding_names: defaults, encode / decode, views, removal
    let rec = binding_names::Rec::default();
    assert_eq!((rec.a, rec.c, rec.inner.x), (1, 17, 5));
    let empty = binding_names::Rec::decode_value(&[], &Path::root(0xF0)).unwrap();
    assert_eq!((empty.a, empty.c, empty.inner.x), (1, 17, 5));
    assert_eq!(binding_names::Root::default().a, 2);
    assert_eq!(<binding_names::Root as ContractState>::state_key(), &[0xF4]);
    let mut recs: Map<u8, binding_names::Rec> = Map::__bound_at(Path::root(0xF0).field(1));
    recs.try_insert(1, binding_names::Rec::default()).unwrap();
    recs.at_mut(&1).unwrap().inner.kids.insert(2, 3);
    testing::flush();
    assert_eq!(recs.at(&1).inner.kids.get(&2), Some(&3));
    assert_eq!(recs.remove(&1).map(|r| r.a), Some(1));
    testing::flush();

    // array_lengths
    let rec = array_lengths::Rec::decode_value(&[], &Path::root(0xF0)).unwrap();
    assert_eq!((rec.a, rec.b, rec.i), ([7; 3], [0; 1], [0; 8]));
    let mut rec = rec;
    rec.h = [9; 7];
    let back = array_lengths::Rec::decode_value(&rec.encode_value(None), &Path::root(0xF0)).unwrap();
    assert_eq!((back.a, back.h), ([7; 3], [9; 7]));

    // root_const
    assert_eq!(<root_const::Root as ContractState>::state_key(), &[0xF6]);
    assert_eq!(<root_const::Root as Root>::ROOT_BYTE, 0xF6);

    // member_names
    let mut holders: Map<u8, member_names::Holder> = Map::__bound_at(Path::root(0xF0).field(2));
    holders.try_insert(1, member_names::Holder::default()).unwrap();
    holders.at_mut(&1).unwrap().r.kids.insert(4, 5);
    testing::flush();
    assert_eq!(holders.at(&1).r.kids.get(&4), Some(&5));
    assert_eq!(holders.get(&1).map(|h| h.r.__m), Some(0));
}

/// `default` and `default_fn` resolve in an `impl` of the record: `Self::` names the user's own
/// associated items, including one named like the derive's old default const.
mod self_items {
    use near_sdk::near;

    #[near(storage)]
    pub struct S {
        #[key(1, default = Self::X)]
        pub a: u8,
        #[key(2, default = Self::__KEY_DEFAULT_1)]
        pub b: u8,
        #[key(3, default_fn = Self::mk)]
        pub c: u8,
        #[key(4, default = String::new())]
        pub d: String,
    }

    impl S {
        const X: u8 = 4;
        const __KEY_DEFAULT_1: u8 = 9;
        fn mk() -> u8 {
            6
        }
    }
}

#[test]
fn defaults_resolve_in_the_record_impl() {
    let v = self_items::S::default();
    assert_eq!((v.a, v.b, v.c, v.d.as_str()), (4, 9, 6, ""));
}

/// A glob-imported `const _key`: the root hooks' unused key parameter used to be `_key`, an
/// identifier pattern the const captured (E0005, refutable pattern in function argument). It is
/// now `_`.
#[allow(non_upper_case_globals, unused_imports)]
mod glob_key {
    mod defs {
        pub const _key: &[u8] = &[];
    }
    use defs::*;
    use near_sdk::near;

    #[near(contract_state(layout = v2, root = 0xF8))]
    pub struct Root {
        #[key(1)]
        pub n: u8,
    }
}

#[test]
fn root_hooks_ignore_user_consts() {
    use near_sdk::state::{ContractState, testing};

    testing::use_mock();
    assert_eq!(<glob_key::Root as ContractState>::state_key(), &[0xF8]);
    let mut root = glob_key::Root::default();
    root.n = 9;
    root.state_write();
    drop(root);
    testing::begin_call();
    assert_eq!(glob_key::Root::state_read().map(|r| r.n), Some(9));
}

/// `member_names` spelled raw: `r#__m` is the field `__m` and `r#__R` the type `__R`. A raw
/// name used to slip past the name avoidance (E0124 / E0062 for the views' phantom field, E0210
/// for the `ViewField` impls' parent parameter).
#[allow(non_camel_case_types)]
mod raw_member_names {
    use near_sdk::near;
    use near_sdk::state::Map;

    #[near(storage)]
    #[derive(Default)]
    pub struct r#__R {
        #[key(1)]
        pub r#__m: u8,
        #[key(2)]
        pub kids: Map<u8, u8>,
    }

    /// `r#__R` nested, so its `ViewField<'a, Holder>` impl is used
    #[near(storage)]
    #[derive(Default)]
    pub struct Holder {
        #[key(1)]
        pub r: r#__R,
    }

    #[near(contract_state(layout = v2, root = 0xF9))]
    pub struct Root {
        #[key(1)]
        pub recs: Map<u8, r#__R>,
        #[key(2)]
        pub holders: Map<u8, Holder>,
    }
}

#[test]
fn raw_names_are_avoided() {
    use near_sdk::state::{Map, Path, testing};
    use raw_member_names::{__R, Holder};

    testing::use_mock();
    let mut recs: Map<u8, __R> = Map::__bound_at(Path::root(0xF0).field(3));
    recs.try_insert(1, __R { __m: 6, ..Default::default() }).unwrap();
    recs.at_mut(&1).unwrap().kids.insert(2, 3);
    let mut holders: Map<u8, Holder> = Map::__bound_at(Path::root(0xF0).field(4));
    holders.try_insert(1, Holder::default()).unwrap();
    holders.at_mut(&1).unwrap().r.kids.insert(4, 5);
    testing::flush();
    assert_eq!(recs.at(&1).kids.get(&2), Some(&3));
    assert_eq!(recs.get(&1).map(|r| r.__m), Some(6));
    assert_eq!(holders.at(&1).r.kids.get(&4), Some(&5));
    assert_eq!(holders.get(&1).map(|h| h.r.__m), Some(0));
}

/// A failing `default = expr` fails `cargo check`, not only `cargo build`, even when the record
/// is never used. trybuild runs `cargo check` when every case is `compile_fail`. The `.stderr`
/// snapshots are from the pinned CI toolchain (1.95); the floating-stable CI row skips this test,
/// as it does `store_performance_tests`. Re-record with `TRYBUILD=overwrite cargo +1.95 test`.
#[rustversion::stable]
#[test]
fn failing_defaults_fail_cargo_check() {
    trybuild::TestCases::new().compile_fail("compilation_tests/storage_v2/*.rs");
}
