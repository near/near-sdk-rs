use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde_json::json;
use near_sdk::{env, near_bindgen, require, Gas, PromiseResult};

// Prepaid gas for a single (not inclusive of recursion) `factorial` call.
const FACTORIAL_CALL_GAS: Gas = Gas(20_000_000_000_000);

// Prepaid gas for a single `factorial_mult` call.
const FACTORIAL_MULT_CALL_GAS: Gas = Gas(10_000_000_000_000);

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

#[near_bindgen]
impl CrossContract {
    pub fn factorial(&self, n: u32) {
        if n <= 1 {
            env::value_return(&borsh::to_vec(&1u32).unwrap());
            return;
        }
        let account_id = env::current_account_id();
        let prepaid_gas = env::prepaid_gas() - FACTORIAL_CALL_GAS;
        let promise0 = env::promise_create(
            account_id.clone(),
            "factorial",
            json!({ "n": n - 1 }).to_string().as_bytes(),
            0,
            prepaid_gas - FACTORIAL_MULT_CALL_GAS,
        );
        let promise1 = env::promise_then(
            promise0,
            account_id,
            "factorial_mult",
            json!({ "n": n }).to_string().as_bytes(),
            0,
            FACTORIAL_MULT_CALL_GAS,
        );
        env::promise_return(promise1);
    }

    /// Used for callbacks only. Multiplies current factorial result by the next value. Panics if
    /// it is not called by the contract itself.
    pub fn factorial_mult(&self, n: u32) {
        require!(env::current_account_id() == env::predecessor_account_id());
        require!(env::promise_results_count() == 1);
        let cur: u32 = match env::promise_result(0) {
            PromiseResult::Successful(x) => BorshDeserialize::try_from_slice(&x).unwrap(),
            _ => env::panic_str("Promise with index 0 failed"),
        };
        env::value_return(&borsh::to_vec(&(cur * n)).unwrap());
    }
}
