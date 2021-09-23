use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, ext_contract, json_types::U128, log, near_bindgen, AccountId, Gas, Promise, PromiseOrValue,
};

const TGAS: Gas = Gas(1_000_000_000_000);

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

// One can provide a name, e.g. `ext` to use for generated methods.
#[ext_contract(ext)]
pub trait ExtCrossContract {
    fn internal_merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>>;
    fn finalize_merge_sort(
        &self,
        #[callback_unwrap]
        #[serializer(borsh)]
        res: Vec<u8>,
    ) -> Vec<u8>;
    fn merge(
        &self,
        #[callback_unwrap]
        #[serializer(borsh)]
        data0: Vec<u8>,
        #[callback_unwrap]
        #[serializer(borsh)]
        data1: Vec<u8>,
    ) -> Vec<u8>;
}

// If the name is not provided, the namespace for generated methods in derived by applying snake
// case to the trait name, e.g. ext_status_message.
#[ext_contract]
pub trait ExtStatusMessage {
    fn set_status(&mut self, message: String);
    fn get_status(&self, account_id: AccountId) -> Option<String>;
}

#[near_bindgen]
impl CrossContract {
    pub fn deploy_status_message(&self, account_id: AccountId, amount: U128) {
        let status_id: AccountId =
            format!("{}.{}", account_id, env::current_account_id()).parse().unwrap();

        Promise::new(status_id)
            .create_account()
            .transfer(amount.0)
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(
                include_bytes!("../../status-message/res/status_message.wasm").to_vec(),
            );
    }

    // external interface, uses JSON serialization
    pub fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        match self.internal_merge_sort(arr) {
            PromiseOrValue::Promise(p) => {
                p.then(ext::finalize_merge_sort(env::current_account_id(), 0, TGAS * 2)).into()
            }
            x => x,
        }
    }

    #[private]
    pub fn finalize_merge_sort(
        &self,
        #[callback_unwrap]
        #[serializer(borsh)]
        res: Vec<u8>,
    ) -> Vec<u8> {
        res
    }

    #[result_serializer(borsh)]
    #[private]
    pub fn internal_merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        if arr.len() <= 1 {
            return PromiseOrValue::Value(arr);
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let account_id = env::current_account_id();
        let gas_to_pass = match pivot {
            1 => TGAS * 1,
            2 => TGAS * 40,
            // TODO: make work with input arrays of length 5, maybe 6
            _ => env::panic_str("Cannot sort arrays larger than length=4 due to gas limits"),
        };
        log!(
            "MERGE_SORT arr={:?}, gas={:?}Tgas, gas_to_pass={:?}Tgas",
            arr,
            env::prepaid_gas().0 / 1_000_000_000_000,
            gas_to_pass.0 / 1_000_000_000_000
        );

        ext::internal_merge_sort(arr0, account_id.clone(), 0, gas_to_pass)
            .and(ext::internal_merge_sort(arr1, account_id.clone(), 0, gas_to_pass))
            .then(ext::merge(account_id, 0, TGAS))
            .into()
    }

    fn internal_merge(&self, arr0: Vec<u8>, arr1: Vec<u8>) -> Vec<u8> {
        let mut i = 0usize;
        let mut j = 0usize;
        let mut result = vec![];
        loop {
            if i == arr0.len() {
                result.extend(&arr1[j..]);
                break;
            }
            if j == arr1.len() {
                result.extend(&arr0[i..]);
                break;
            }
            if arr0[i] < arr1[j] {
                result.push(arr0[i]);
                i += 1;
            } else {
                result.push(arr1[j]);
                j += 1;
            }
        }
        result
    }

    /// Used for callbacks only. Merges two sorted arrays into one. Panics if it is not called by
    /// the contract itself.
    #[result_serializer(borsh)]
    #[private]
    pub fn merge(
        &self,
        #[callback_unwrap]
        #[serializer(borsh)]
        data0: Vec<u8>,
        #[callback_unwrap]
        #[serializer(borsh)]
        data1: Vec<u8>,
    ) -> Vec<u8> {
        log!(
            "MERGE data0={:?}, data1={:?}, gas={:?}Ggas",
            data0,
            data1,
            env::prepaid_gas().0 / 1_000_000_000
        );
        let result = self.internal_merge(data0, data1);
        result
    }

    //    /// Alternative implementation of merge that demonstrates usage of callback_vec. Uncomment
    //    /// to use.
    //    pub fn merge(&self, #[callback_vec] #[serializer(borsh)] arrs: &mut Vec<Vec<u8>>) -> Vec<u8> {
    //        assert_eq!(env::current_account_id(), env::predecessor_account_id());
    //        self.internal_merge(arrs.pop().unwrap(), arrs.pop().unwrap())
    //    }

    pub fn simple_call(&mut self, account_id: AccountId, message: String) {
        ext_status_message::set_status(message, account_id, 0, env::prepaid_gas() / 2);
    }
    pub fn complex_call(&mut self, account_id: AccountId, message: String) -> Promise {
        // 1) call status_message to record a message from the signer.
        // 2) call status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        let prepaid_gas = env::prepaid_gas();
        log!("complex_call");
        ext_status_message::set_status(message, account_id.clone(), 0, prepaid_gas / 3).then(
            ext_status_message::get_status(
                env::signer_account_id(),
                account_id,
                0,
                prepaid_gas / 3,
            ),
        )
    }

    pub fn transfer_money(&mut self, account_id: AccountId, amount: u64) {
        Promise::new(account_id).transfer(amount as u128);
    }
}
