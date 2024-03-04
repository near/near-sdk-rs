use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

type MyResult = Result<u32, &'static str>;

#[near(contract_state)]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Contract {
    value: u32,
}

#[near]
impl Contract {
    #[handle_result(aliased)]
    pub fn fun(&self) -> MyResult {
        Err("error")
    }
}

fn main() {}
