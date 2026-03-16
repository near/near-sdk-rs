//! Verify that `#[near]` macro automatically suppresses `clippy::ptr_arg` for
//! methods with reference arguments, since the macro requires owned/sized types
//! in the generated `Input` struct and users cannot follow clippy's suggestion.
//! See https://github.com/near/near-sdk-rs/issues/1551

use near_sdk::near;

#[near(contract_state)]
#[derive(Default)]
struct Contract {
    data: Vec<String>,
}

#[near]
impl Contract {
    pub fn with_vec_ref(&self, arg: &Vec<String>) -> u32 {
        arg.len() as u32
    }

    pub fn with_string_ref(&self, arg: &String) -> String {
        arg.clone()
    }
}

fn main() {}
