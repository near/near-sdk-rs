use near_sdk::serde_json;
use near_sdk::{env, near, require, Gas, NearToken, PromiseResult};

// Prepaid gas for a single (not inclusive of recursion) `factorial` call.
const FACTORIAL_CALL_GAS: Gas = Gas::from_tgas(20);

// Prepaid gas for a single `factorial_mult` call.
const FACTORIAL_MULT_CALL_GAS: Gas = Gas::from_tgas(10);

#[derive(Default)]
#[near(contract_state)]
pub struct CrossContract {}

#[near]
impl CrossContract {
    pub fn factorial(&self, n: u32) {
        if n <= 1 {
            env::value_return(&serde_json::to_vec(&1u32).unwrap());
            return;
        }
        let account_id = env::current_account_id();
        let prepaid_gas = env::prepaid_gas().saturating_sub(FACTORIAL_CALL_GAS);
        let promise0 = env::promise_create(
            account_id.clone(),
            "factorial",
            &serde_json::to_vec(&(n - 1,)).unwrap(),
            NearToken::from_near(0),
            prepaid_gas.saturating_sub(FACTORIAL_MULT_CALL_GAS),
        );
        let promise1 = env::promise_then(
            promise0,
            account_id,
            "factorial_mult",
            &serde_json::to_vec(&(n,)).unwrap(),
            NearToken::from_near(0),
            FACTORIAL_MULT_CALL_GAS,
        );
        env::promise_return(promise1);
    }

    /// Used for callbacks only. Multiplies current factorial result by the next value. Panics if
    /// it is not called by the contract itself.
    pub fn factorial_mult(&self, n: u32) {
        require!(env::current_account_id() == env::predecessor_account_id());
        require!(env::promise_results_count() == 1);
        let Ok(cur) = env::promise_result_bounded(0, 15) else {
            env::panic_str("Promise with index 0 failed or returned too long result");
        };
        env::value_return(&serde_json::to_vec(&(cur * n)).unwrap());
    }
}
