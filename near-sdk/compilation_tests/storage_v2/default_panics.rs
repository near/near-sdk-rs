//! A `default` whose const evaluation panics fails `cargo check`, though `S` is never used.
mod defs {
    pub const fn boom() -> u8 {
        panic!("bad default")
    }
}

#[near_sdk::near(storage)]
struct S {
    #[key(1, default = defs::boom())]
    a: u8,
}

fn main() {}
