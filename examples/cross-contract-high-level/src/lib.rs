use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::{env, near_bindgen, PromiseResult, Promise, PromiseOrValue};
use serde_json::json;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}


use near_bindgen::{Gas, Balance, AccountId};
// #[ext-contract(a0)]
pub trait ExternalCrossContract {
    fn merge_sort(&self, arr: Vec<u8>);
    fn merge(&self) -> Vec<u8>;
}

mod a0 {
    use super::*;
    pub fn merge_sort(arr: Vec<u8>, balance: Balance, gas: Gas) -> Promise {
        Promise::new(
            env::current_account_id(),
            b"merge_sort".to_vec(),
            json!({ "arr": arr }).to_string().as_bytes().to_vec(),
            balance,
            gas,
            None,
        )
    }

    pub fn merge(balance: Balance, gas: Gas) -> Promise {
        Promise::new(
            env::current_account_id(),
            b"merge".to_vec(),
            vec![],
            balance,
            gas,
            None,
        )
    }
}

// callback_input!(Vec<u8>, Vec<u8>);
pub fn merge_input() -> (Vec<u8>, Vec<u8>) {
    let data0: Vec<u8> = match env::promise_result(0) {
        PromiseResult::Successful(x) => x,
        _ => unreachable!()
    };
    let data1: Vec<u8> = match env::promise_result(1) {
        PromiseResult::Successful(x) => x,
        _ => unreachable!()
    };
    (serde_json::from_slice(&data0).unwrap(), serde_json::from_slice(&data1).unwrap())
}

#[near_bindgen]
impl CrossContract {
    pub fn deploy_status_message(&self, account_id: String, amount: u64) {
        let promise_idx = env::promise_batch_create(account_id);
        env::promise_batch_action_create_account(promise_idx);
        env::promise_batch_action_transfer(promise_idx, amount as u128);
        env::promise_batch_action_add_key_with_full_access(promise_idx, env::signer_account_pk(), 0);
        let code: &[u8] = include_bytes!("../../status-message/res/status_message.wasm");
        env::promise_batch_action_deploy_contract(promise_idx, code);
    }

    pub fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        if arr.len() <= 1 {
            return PromiseOrValue::Value(arr);
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let account_id = env::current_account_id();
        let prepaid_gas = env::prepaid_gas();

        a0::merge_sort(arr0, 0, prepaid_gas/4)
            .join(a0::merge_sort(arr1, 0, prepaid_gas/4))
            .and_then(a0::merge(0, prepaid_gas/4)).into()
    }

    /// Used for callbacks only. Merges two sorted arrays into one. Panics if it is not called by
    /// the contract itself.
    pub fn merge(&self) -> Vec<u8> {
        assert_eq!(env::current_account_id(), env::predecessor_account_id());
        let (data0, data1) = merge_input();
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
            }  else {
                result.push(data1[j]);
                j += 1;
            }
        }
        result
    }

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
