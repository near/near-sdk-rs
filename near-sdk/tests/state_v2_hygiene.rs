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
