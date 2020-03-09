pub use near_bindgen_macros::{
    callback, callback_vec, ext_contract, init, near_bindgen, result_serializer, serializer, metadata
};

pub mod collections;
mod environment;
pub use environment::env;

mod promise;
pub use promise::{Promise, PromiseOrValue};

mod metadata;
pub use metadata::{Metadata, MethodMetadata};

#[cfg(not(target_arch = "wasm32"))]
pub use environment::mocked_blockchain::MockedBlockchain;
#[cfg(not(target_arch = "wasm32"))]
pub use near_runtime_fees::RuntimeFeesConfig;
pub use near_vm_logic::types::*;
#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::VMConfig;
#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::VMContext;

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! testing_env {
    ($context:expr, $config:expr, $fee_config:expr) => {
        let storage = match near_bindgen::env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };

        near_bindgen::env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            $context,
            $config,
            $fee_config,
            vec![],
            storage,
        )));
    };
    ($context:expr) => {
        testing_env!($context, Default::default(), Default::default());
    };
}

pub use environment::blockchain_interface::BlockchainInterface;
