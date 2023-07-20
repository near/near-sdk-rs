use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::near_bindgen;

type MyResult = Result<u32, &'static str>;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Contract {
    value: u32,
}

#[near_bindgen]
impl Contract {
    #[handle_result]
    pub fn fun(&self) -> MyResult {
        Err("error")
    }
}

fn main() {}
