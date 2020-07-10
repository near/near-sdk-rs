#[cfg(test)]
extern crate quickcheck;

pub use near_sdk_macros::{
    callback, callback_vec, ext_contract, init, metadata, near_bindgen, result_serializer,
    serializer,
};

pub mod collections;
mod environment;
pub use environment::env;

mod promise;
pub use promise::{Promise, PromiseOrValue};

mod metadata;
pub use metadata::{Metadata, MethodMetadata};

pub mod json_types;

pub use environment::mocked_blockchain::MockedBlockchain;
pub use near_runtime_fees::RuntimeFeesConfig;
pub use near_vm_logic::types::*;
pub use near_vm_logic::VMConfig;
pub use near_vm_logic::VMContext;

#[macro_export]
macro_rules! testing_env {
    ($context:expr, $config:expr, $fee_config:expr, $validator:expr) => {
        let storage = match near_sdk::env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };

        near_sdk::env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            $context,
            $config,
            $fee_config,
            vec![],
            storage,
            $validator,
        )));
    };
    ($context:expr, $config:expr, $fee_config:expr) => {
        testing_env!($context, $config, $fee_config, Default::default());
    };
    ($context:expr) => {
        testing_env!($context, Default::default(), Default::default());
    };
}

pub use environment::blockchain_interface::BlockchainInterface;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(crate) mod test_utils;

// Exporting common crates

#[doc(hidden)]
pub use borsh;

#[doc(hidden)]
pub use base64;

#[doc(hidden)]
pub use bs58;

#[doc(hidden)]
pub use serde;

#[doc(hidden)]
pub use serde_json;

#[doc(hidden)]
pub use wee_alloc;
