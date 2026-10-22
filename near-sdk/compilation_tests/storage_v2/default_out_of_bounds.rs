//! A `default` indexing out of bounds fails `cargo check`, though `S` is never used.
#[near_sdk::near(storage)]
struct S {
    #[key(1, default = [1u8][1])]
    a: u8,
}

fn main() {}
