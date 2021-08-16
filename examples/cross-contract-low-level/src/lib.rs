use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde_json::{self, json};
use near_sdk::{env, near_bindgen, require, AccountId, Gas, PromiseResult};

// Prepaid gas for making a single simple call.
const SINGLE_CALL_GAS: Gas = Gas(200000000000000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CrossContract {
    checked_promise: bool,
}

impl Default for CrossContract {
    fn default() -> Self {
        CrossContract { checked_promise: false }
    }
}

#[near_bindgen]
impl CrossContract {
    pub fn deploy_status_message(&self, account_id: AccountId, amount: U128) {
        let promise_idx = env::promise_batch_create(&account_id);
        env::promise_batch_action_create_account(promise_idx);
        env::promise_batch_action_transfer(promise_idx, amount.0);
        env::promise_batch_action_add_key_with_full_access(
            promise_idx,
            &env::signer_account_pk(),
            0,
        );
        let code: &[u8] = include_bytes!("../../status-message/res/status_message.wasm");
        env::promise_batch_action_deploy_contract(promise_idx, code);
    }

    pub fn merge_sort(&self, arr: Vec<u8>) {
        if arr.len() <= 1 {
            env::value_return(&serde_json::to_vec(&arr).unwrap());
            return;
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let account_id = env::current_account_id();
        let prepaid_gas = env::prepaid_gas();
        let promise0 = env::promise_create(
            account_id.clone(),
            "merge_sort",
            json!({ "arr": arr0 }).to_string().as_bytes(),
            0,
            prepaid_gas / 4,
        );
        let promise1 = env::promise_create(
            account_id.clone(),
            "merge_sort",
            json!({ "arr": arr1 }).to_string().as_bytes(),
            0,
            prepaid_gas / 4,
        );
        let promise2 = env::promise_and(&[promise0, promise1]);
        let promise3 =
            env::promise_then(promise2, account_id.clone(), "merge", &[], 0, prepaid_gas / 4);
        env::promise_return(promise3);
    }

    /// Used for callbacks only. Merges two sorted arrays into one. Panics if it is not called by
    /// the contract itself.
    pub fn merge(&self) -> Vec<u8> {
        require!(env::current_account_id() == env::predecessor_account_id());
        require!(env::promise_results_count() == 2);
        let data0: Vec<u8> = match env::promise_result(0) {
            PromiseResult::Successful(x) => x,
            _ => env::panic_str("Promise with index 0 failed"),
        };
        let data0: Vec<u8> = serde_json::from_slice(&data0).unwrap();
        let data1: Vec<u8> = match env::promise_result(1) {
            PromiseResult::Successful(x) => x,
            _ => env::panic_str("Promise with index 1 failed"),
        };
        let data1: Vec<u8> = serde_json::from_slice(&data1).unwrap();
        let mut i = 0usize;
        let mut j = 0usize;
        let mut result = vec![];
        loop {
            if i == data0.len() {
                result.extend(&data1[j..]);
                break;
            }
            if j == data1.len() {
                result.extend(&data0[i..]);
                break;
            }
            if data0[i] < data1[j] {
                result.push(data0[i]);
                i += 1;
            } else {
                result.push(data1[j]);
                j += 1;
            }
        }
        result
    }

    pub fn simple_call(&mut self, account_id: AccountId, message: String) {
        env::promise_create(
            account_id,
            "set_status",
            json!({ "message": message }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );
    }
    pub fn complex_call(&mut self, account_id: AccountId, message: String) {
        // 1) call status_message to record a message from the signer.
        // 2) check that the promise succeed
        // 3) call status_message to retrieve the message of the signer.
        // 4) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        let promise0 = env::promise_create(
            account_id.clone(),
            "set_status",
            json!({ "message": message }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );
        let promise1 = env::promise_then(
            promise0,
            env::current_account_id(),
            "check_promise",
            json!({}).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );
        let promise2 = env::promise_then(
            promise1,
            account_id,
            "get_status",
            json!({ "account_id": env::signer_account_id() }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );
        env::promise_return(promise2);
    }

    pub fn check_promise(&mut self) {
        match env::promise_result(0) {
            PromiseResult::Successful(_) => {
                env::log_str("Check_promise successful");
                self.checked_promise = true;
            }
            _ => env::panic_str("Promise with index 0 failed"),
        };
    }

    pub fn transfer_money(&mut self, account_id: AccountId, amount: u64) {
        let promise_idx = env::promise_batch_create(&account_id);
        env::promise_batch_action_transfer(promise_idx, amount as u128);
    }

    pub fn promise_checked(&self) -> bool {
        self.checked_promise
    }
}
