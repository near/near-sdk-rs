use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    //    callback,
    //    callback_vec,
    env,
    ext_contract,
    near_bindgen,
    Promise,
    PromiseOrValue,
};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

// Prepaid gas for making a single simple call.
const SINGLE_CALL_GAS: u64 = 200_000_000_000_000;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

// One can provide a name, e.g. `ext` to use for generated methods.
#[ext_contract(ext)]
pub trait ExtCrossContract {
    fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>>;
    fn merge(
        &self,
        #[callback]
        #[serializer(borsh)]
        data0: Vec<u8>,
        #[callback]
        #[serializer(borsh)]
        data1: Vec<u8>,
    ) -> Vec<u8>;
}

// If the name is not provided, the namespace for generated methods in derived by applying snake
// case to the trait name, e.g. ext_status_message.
#[ext_contract]
pub trait ExtStatusMessage {
    fn set_status(&mut self, message: String);
    fn get_status(&self, account_id: String) -> Option<String>;
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

    #[result_serializer(borsh)]
    pub fn merge_sort(&self, arr: Vec<u8>) -> PromiseOrValue<Vec<u8>> {
        if arr.len() <= 1 {
            return PromiseOrValue::Value(arr);
        }
        let pivot = arr.len() / 2;
        let arr0 = arr[..pivot].to_vec();
        let arr1 = arr[pivot..].to_vec();
        let prepaid_gas = env::prepaid_gas();
        let account_id = env::current_account_id();

        ext::merge_sort(arr0, &account_id, 0, prepaid_gas / 4)
            .and(ext::merge_sort(arr1, &account_id, 0, prepaid_gas / 4))
            .then(ext::merge(&account_id, 0, prepaid_gas / 4))
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
        #[callback]
        #[serializer(borsh)]
        data0: Vec<u8>,
        #[callback]
        #[serializer(borsh)]
        data1: Vec<u8>,
    ) -> Vec<u8> {
        env::log(format!("Received {:?} and {:?}", data0, data1).as_bytes());
        let result = self.internal_merge(data0, data1);
        env::log(format!("Merged {:?}", result).as_bytes());
        result
    }

    //    /// Alternative implementation of merge that demonstrates usage of callback_vec. Uncomment
    //    /// to use.
    //    pub fn merge(&self, #[callback_vec] #[serializer(borsh)] arrs: &mut Vec<Vec<u8>>) -> Vec<u8> {
    //        assert_eq!(env::current_account_id(), env::predecessor_account_id());
    //        self.internal_merge(arrs.pop().unwrap(), arrs.pop().unwrap())
    //    }

    pub fn simple_call(&mut self, account_id: String, message: String) {
        ext_status_message::set_status(message, &account_id, 0, SINGLE_CALL_GAS);
    }
    pub fn complex_call(&mut self, account_id: String, message: String) -> Promise {
        // 1) call status_message to record a message from the signer.
        // 2) call status_message to retrieve the message of the signer.
        // 3) return that message as its own result.
        // Note, for a contract to simply call another contract (1) is sufficient.
        ext_status_message::set_status(message, &account_id, 0, SINGLE_CALL_GAS).then(
            ext_status_message::get_status(
                env::signer_account_id(),
                &account_id,
                0,
                SINGLE_CALL_GAS,
            ),
        )
    }

    pub fn transfer_money(&mut self, account_id: String, amount: u64) {
        Promise::new(account_id).transfer(amount as u128);
    }
}
