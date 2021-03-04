use crate::env;

#[allow(dead_code)]
pub mod test_env;

mod context;
pub use context::{accounts, testing_env_with_promise_results, VMContextBuilder};
use near_vm_logic::mocks::mock_external::Receipt;

#[macro_export]
macro_rules! testing_env {
    ($context:expr, $config:expr, $fee_config:expr, $validator:expr, $promise_results:expr) => {
        near_sdk::env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            $context,
            $config,
            $fee_config,
            $promise_results,
            match near_sdk::env::take_blockchain_interface() {
                Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
                None => Default::default(),
            },
            $validator,
        )));
    };
    ($context:expr, $config:expr, $fee_config:expr, $validator:expr) => {
        testing_env!($context, $config, $fee_config, $validator, Default::default());
    };

    ($context:expr, $config:expr, $fee_config:expr) => {
        testing_env!($context, $config, $fee_config, Default::default());
    };
    ($context:expr) => {
        testing_env!($context, Default::default(), Default::default());
    };
}

#[allow(dead_code)]
/// Returns a copy of logs from VMLogic. Only available in unit tests.
pub fn get_logs() -> Vec<String> {
    let blockchain_interface =
        env::take_blockchain_interface().expect("Blockchain interface is not set");
    let logs = blockchain_interface
        .as_mocked_blockchain()
        .expect("MockedBlockchain interface expected")
        .logs();
    env::set_blockchain_interface(blockchain_interface);
    logs
}

/// Accessing receipts created by the contract. Only available in unit tests.
#[allow(dead_code)]
pub fn get_created_receipts() -> Vec<Receipt> {
    let blockchain_interface =
        env::take_blockchain_interface().expect("Blockchain interface is not set");
    let receipts = blockchain_interface
        .as_mocked_blockchain()
        .expect("MockedBlockchain interface expected")
        .created_receipts()
        .clone();
    env::set_blockchain_interface(blockchain_interface);
    receipts
}

/// Objects stored on the trie directly should have identifiers. If identifier is not provided
/// explicitly than `Default` trait would use this index to generate an id.
#[allow(dead_code)]
pub(crate) static mut NEXT_TRIE_OBJECT_INDEX: u64 = 0;
/// Get next id of the object stored on trie.
#[allow(dead_code)]
pub(crate) fn next_trie_id() -> Vec<u8> {
    unsafe {
        let id = NEXT_TRIE_OBJECT_INDEX;
        NEXT_TRIE_OBJECT_INDEX += 1;
        id.to_le_bytes().to_vec()
    }
}
