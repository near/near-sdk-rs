use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::{env, ext_contract, near_bindgen, Promise, PromiseOrValue, PromiseResult};
use serde_json::json;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

#[ext_contract]
pub trait A0 {
    fn merge_sort(&self, arr: Vec<u8>);
    fn merge(&self) -> Vec<u8>;
}

#[ext_contract]
pub trait S0 {
    fn set_status(&mut self, message: String);
    fn get_status(&self, account_id: String) -> Option<String>;
}

// #[callback_args(arr0, arr1)]
// #[callback_args_vec(arr)]
pub fn merge_input() -> (Vec<u8>, Vec<u8>) {
    let data0: Vec<u8> = match env::promise_result(0) {
        PromiseResult::Successful(x) => x,
        _ => unreachable!(),
    };
    let data1: Vec<u8> = match env::promise_result(1) {
        PromiseResult::Successful(x) => x,
        _ => unreachable!(),
    };
    (serde_json::from_slice(&data0).unwrap(), serde_json::from_slice(&data1).unwrap())
}

#[near_bindgen]
impl CrossContract {
    pub fn deploy_status_message(&self, account_id: String, amount: u64) {
        Promise::new(account_id)
            .create_account()
            .transfer(amount as u128)
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(
                include_bytes!("../../status-message/res/status_message.wasm").to_vec(),
            );
    }

    pub fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        if arr.len() <= 1 {
            return PromiseOrValue::Value(arr);
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let prepaid_gas = env::prepaid_gas();
        let account_id = env::current_account_id();

        a0::merge_sort(arr0, &account_id, 0, prepaid_gas / 4)
            .join(a0::merge_sort(arr1, &account_id, 0, prepaid_gas / 4))
            .and_then(a0::merge(&account_id, 0, prepaid_gas / 4))
            .into()
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
            } else {
                result.push(data1[j]);
                j += 1;
            }
        }
        result
    }

    pub fn simple_call(&mut self, account_id: String, message: String) {
        s0::set_status(message, &account_id, 0, 1_000_000);
    }
    pub fn complex_call(&mut self, account_id: String, message: String) -> Promise {
        // 1) call status_message to record a message from the signer.
        // 2) call status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        s0::set_status(message, &account_id, 0, 1_000_000).and_then(s0::get_status(
            env::signer_account_id(),
            &account_id,
            0,
            1_000_000,
        ))
    }

    pub fn transfer_money(&mut self, account_id: String, amount: u64) {
        Promise::new(account_id).transfer(amount as u128);
    }
}
