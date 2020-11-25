use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, ValidAccountId};
use near_sdk::{env, log, near_bindgen, serde_json, AccountId, Balance, Gas, Promise};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct PromiseBob {}

const NO_DEPOSIT: Balance = 0;

const BASIC_GAS: Gas = 5_000_000_000_000;

fn log_it(s: &str) {
    log!(
        "#{}   I'm @{}. Called by @{}. {}",
        env::block_index(),
        env::current_account_id(),
        env::predecessor_account_id(),
        s
    );
}

#[near_bindgen]
impl PromiseBob {
    pub fn get_data(&self) -> String {
        log_it("get_data");
        "123".to_string()
    }
}
