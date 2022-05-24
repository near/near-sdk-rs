use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env;
use near_sdk::{log, near_bindgen, PromiseOrValue};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

#[near_bindgen]
impl CrossContract {
    pub fn factorial(&self, n: u32) -> PromiseOrValue<u32> {
        if n <= 1 {
            return PromiseOrValue::Value(1);
        }
        let account_id = env::current_account_id();

        Self::ext(account_id.clone())
            .with_unused_gas_weight(6)
            .factorial(n - 1)
            .then(Self::ext(account_id).factorial_mult(n))
            .into()
    }

    /// Used for callbacks only. Multiplies current factorial result by the next value. Panics if
    /// it is not called by the contract itself.
    #[private]
    pub fn factorial_mult(&self, n: u32, #[callback_unwrap] cur: u32) -> u32 {
        log!("Received {:?} and {:?}", n, cur);
        let result = n * cur;
        log!("Multiplied {:?}", result.clone());
        result
    }
}
