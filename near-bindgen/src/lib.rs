pub use near_bindgen_macros::near_bindgen;

pub mod collections;
mod environment;
pub use environment::blockchain_interface::BlockchainInterface;
pub use environment::environment::Environment;
pub use environment::mocked_blockchain::MockedBlockchain;


#[cfg(feature = "testing")]
pub use near_vm_logic::VMLogic;
#[cfg(feature = "testing")]
pub use near_vm_logic::mocks::mock_memory::MockedMemory;
#[cfg(feature = "testing")]
pub use near_vm_logic::mocks::mock_external::MockedExternal;
#[cfg(feature = "testing")]
pub use near_vm_logic::VMContext;
#[cfg(feature = "testing")]
pub use near_vm_logic::Config;

/// Sets up the testing environment for the near-bindgen tests.
/// # Example:
///  ```rust
///     let context = VMContext {
///            current_account_id: "alice.near".to_string(),
///            signer_account_id: "bob.near".to_string(),
///            signer_account_pk: vec![0, 1, 2],
///            predecessor_account_id: "carol.near".to_string(),
///            input: vec![],
///            block_index: 0,
///            account_balance: 0,
///            storage_usage: 0,
///            attached_deposit: 0,
///            prepaid_gas: 10u64.pow(9),
///            random_seed: vec![0, 1, 2],
///            free_of_charge: false,
///            output_data_receivers: vec![],
///        };
///     testing_env!(context, Config::default());
///  ```
#[cfg(feature = "testing")]
#[macro_export]
macro_rules! testing_env {
    ($env: ident, $context:ident, $config:expr) => {
        let mut ext = $crate::MockedExternal::new();
        let config = $crate::Config::default();
        let promise_results = vec![];
        let mut memory = $crate::MockedMemory::new();
        let logic = $crate::VMLogic::new(&mut ext, $context, &$config, &promise_results, &mut memory);
        let mut $env = $crate::Environment::new(Box::new(MockedBlockchain::new(logic)));
    }
}
