pub use near_bindgen_macros::{callback_args, callback_args_vec, ext_contract, near_bindgen};

pub mod collections;
mod environment;
pub use environment::env;

mod promise;
pub use promise::{Promise, PromiseOrValue};

#[cfg(feature = "testing")]
pub use environment::mocked_blockchain::MockedBlockchain;
pub use near_vm_logic::types::*;
#[cfg(feature = "testing")]
pub use near_vm_logic::Config;
#[cfg(feature = "testing")]
pub use near_vm_logic::VMContext;

#[cfg(feature = "testing")]
#[macro_export]
macro_rules! testing_env {
    ($context:expr, $config:expr) => {
        let storage = match near_bindgen::env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };

        near_bindgen::env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            $context,
            $config,
            vec![],
            storage,
        )));
    };
}

pub use environment::blockchain_interface::BlockchainInterface;
