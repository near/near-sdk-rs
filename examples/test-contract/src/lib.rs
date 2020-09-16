use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct TestContract {}

impl Default for TestContract {
    fn default() -> Self {
        Self {}
    }
}

#[near_bindgen]
impl TestContract {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {}
    }

    #[init]
    pub fn migrate() -> Self {
        unimplemented!("Unimplemented");
    }

    pub fn test_panic_macro(&mut self) {
        panic!("PANIC!");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    #[test]
    #[should_panic(expected = "PANIC!")]
    fn test_panic() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let mut contract = TestContract::new();
        contract.test_panic_macro();
    }
}
