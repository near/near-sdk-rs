use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde_json;
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
            env::value_return(&serde_json::to_vec(&1u32).unwrap());
            return;
        }
        let account_id = env::current_account_id();
        let prepaid_gas = env::prepaid_gas() - FACTORIAL_CALL_GAS;
        let promise0 = env::promise_create(
            account_id.clone(),
            "factorial",
            &serde_json::to_vec(&(n - 1,)).unwrap(),
            0,
            prepaid_gas - FACTORIAL_MULT_CALL_GAS,
        );
        let promise1 = env::promise_then(
            promise0,
            account_id,
            "factorial_mult",
            &serde_json::to_vec(&(n,)).unwrap(),
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
        let cur = match env::promise_result(0) {
            PromiseResult::Successful(x) => serde_json::from_slice::<u32>(&x).unwrap(),
            _ => env::panic_str("Promise with index 0 failed"),
        };
        env::value_return(&serde_json::to_vec(&(cur * n)).unwrap());
    }
}
