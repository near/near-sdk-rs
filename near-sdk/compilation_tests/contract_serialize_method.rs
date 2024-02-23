use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    log, near_bindgen, PanicOnDefault,
};

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    // See <https://github.com/near/near-sdk-rs/issues/1084#issue-1898868964> for more information.
    pub fn serialize(&mut self) {
        log!("serialize");
    }
}

fn main() {}
