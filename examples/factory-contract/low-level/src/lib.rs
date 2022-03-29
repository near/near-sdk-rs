use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde_json;
use near_sdk::{env, near_bindgen, AccountId, Gas, PromiseResult};

// Prepaid gas for making a single simple call.
const SINGLE_CALL_GAS: Gas = Gas(20_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct FactoryContract {
    checked_promise: bool,
}

impl Default for FactoryContract {
    fn default() -> Self {
        FactoryContract { checked_promise: false }
    }
}

#[near_bindgen]
impl FactoryContract {
    pub fn deploy_status_message(&self, account_id: AccountId, amount: U128) {
        let promise_idx = env::promise_batch_create(&account_id);
        env::promise_batch_action_create_account(promise_idx);
        env::promise_batch_action_transfer(promise_idx, amount.0);
        env::promise_batch_action_add_key_with_full_access(
            promise_idx,
            &env::signer_account_pk(),
            0,
        );
        let code: &[u8] = include_bytes!("../../../status-message/res/status_message.wasm");
        env::promise_batch_action_deploy_contract(promise_idx, code);
    }

    pub fn simple_call(&mut self, account_id: AccountId, message: String) {
        env::promise_create(
            account_id,
            "set_status",
            &serde_json::to_vec(&(message,)).unwrap(),
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
            &serde_json::to_vec(&(message,)).unwrap(),
            0,
            SINGLE_CALL_GAS,
        );
        let promise1 = env::promise_then(
            promise0,
            env::current_account_id(),
            "check_promise",
            &serde_json::to_vec(&()).unwrap(),
            0,
            SINGLE_CALL_GAS,
        );
        let promise2 = env::promise_then(
            promise1,
            account_id,
            "get_status",
            &serde_json::to_vec(&(env::signer_account_id(),)).unwrap(),
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

    pub fn promise_checked(&self) -> bool {
        self.checked_promise
    }
}
