#![feature(const_vec_new)]
use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::{env, near_bindgen};
use serde_json::json;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

#[near_bindgen]
impl CrossContract {
    pub fn simple_call(&mut self, account_id: String, message: String) {
        env::promise_create(
            account_id.clone(),
            b"set_status",
            json!({ "message": message }).to_string().as_bytes(),
            0,
            1_000_000,
        );
    }
    pub fn complex_call(&mut self, account_id: String, message: String) {
        // 1) call status_message to record a message from the signer.
        // 2) call status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        let promise0 = env::promise_create(
            account_id.clone(),
            b"set_status",
            json!({ "message": message }).to_string().as_bytes(),
            0,
            1_000_000,
        );
        let promise1 = env::promise_then(
            promise0,
            account_id,
            b"get_status",
            json!({ "account_id": env::signer_account_id() }).to_string().as_bytes(),
            0,
            1_000_000,
        );
        env::promise_return(promise1);
    }

    pub fn transfer_money(&mut self, account_id: String, amount: u64) {
        let promise_idx = env::promise_batch_create(account_id);
        env::promise_batch_action_transfer(promise_idx, amount as u128);
    }
}
