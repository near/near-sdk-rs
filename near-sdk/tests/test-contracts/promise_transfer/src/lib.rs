//! A contract whose only promise action is a transfer, built with near-sdk's default features.

use near_sdk::{AccountId, NearToken, Promise, near};

#[derive(Default)]
#[near(contract_state)]
struct Payer;

#[near]
impl Payer {
    pub fn pay(&mut self, receiver_id: AccountId) -> Promise {
        Promise::new(receiver_id).transfer(NearToken::from_yoctonear(1))
    }
}
