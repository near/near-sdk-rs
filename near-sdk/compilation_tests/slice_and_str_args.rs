//! Verify that `#[near]` macro supports `&[T]` and `&str` argument types
//! by converting them to owned types (`Vec<T>`, `String`) in the generated
//! `Input` struct, while passing them as references to the actual method.
//! See https://github.com/near/near-sdk-rs/issues/1513

use near_sdk::near;

#[near(contract_state)]
#[derive(Default)]
struct Contract {
    data: Vec<String>,
}

#[near]
impl Contract {
    pub fn with_slice(&self, items: &[String]) -> u32 {
        items.len() as u32
    }

    pub fn with_str(&self, name: &str) -> String {
        name.to_string()
    }

    pub fn with_vec_ref(&self, items: &Vec<String>) -> u32 {
        items.len() as u32
    }

    pub fn with_string_ref(&self, name: &String) -> String {
        name.clone()
    }

    pub fn mixed_args(&self, names: &[String], prefix: &str, count: u32) -> u32 {
        names.len() as u32 + prefix.len() as u32 + count
    }
}

fn main() {}
