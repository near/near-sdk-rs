//! The same at a root, for a type with drop glue.
const fn boom() -> String {
    panic!("bad default")
}

#[near_sdk::near(contract_state(layout = v2))]
pub struct Root {
    #[key(1, default = boom())]
    a: String,
}

fn main() {}
