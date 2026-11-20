#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests;

use near_sdk::{env, near};

#[derive(Default)]
#[near(contract_state)]
pub struct TestContract {
    value: u32,
}

#[near]
impl TestContract {
    #[init]
    pub fn new() -> Self {
        Self { value: 0 }
    }

    /// A private init method that can only be called by the contract itself.
    /// This is useful for factory patterns where a deployed contract should
    /// only be initialized by itself via a scheduled function call.
    #[init]
    #[private]
    pub fn new_private(value: u32) -> Self {
        Self { value }
    }

    #[init(ignore_state)]
    pub fn migrate_state() -> Self {
        #[near]
        struct OldContract {
            // ...
        }

        let _old_contract: OldContract = env::state_read().expect("Old state doesn't exist");

        Self { value: 0 }
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }

    pub fn test_panic_macro(&mut self) {
        panic!("PANIC!");
    }
}
