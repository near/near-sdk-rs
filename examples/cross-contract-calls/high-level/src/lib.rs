use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Gas};
use near_sdk::{ext_contract, log, near_bindgen, PromiseOrValue};

// Prepaid gas for a single (not inclusive of recursion) `factorial` call.
const FACTORIAL_CALL_GAS: Gas = Gas(20_000_000_000_000);

// Prepaid gas for a single `factorial_mult` call.
const FACTORIAL_MULT_CALL_GAS: Gas = Gas(10_000_000_000_000);

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct CrossContract {}

// One can provide a name, e.g. `ext` to use for generated methods.
#[ext_contract(ext)]
pub trait ExtCrossContract {
    fn factorial(&self, n: u32) -> PromiseOrValue<u32>;
    fn factorial_mult(&self, n: u32, #[callback_unwrap] cur: u32) -> u32;
}

#[near_bindgen]
impl CrossContract {
    pub fn factorial(&self, n: u32) -> PromiseOrValue<u32> {
        if n <= 1 {
            return PromiseOrValue::Value(1);
        }
        let account_id = env::current_account_id();
        let prepaid_gas = env::prepaid_gas() - FACTORIAL_CALL_GAS;

        ext::factorial(n - 1, account_id.clone(), 0, prepaid_gas - FACTORIAL_MULT_CALL_GAS)
            .then(ext::factorial_mult(n, account_id, 0, FACTORIAL_MULT_CALL_GAS))
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
