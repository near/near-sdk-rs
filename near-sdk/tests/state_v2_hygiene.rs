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
