#[cfg(test)]
extern crate quickcheck;

pub use near_sdk_macros::{
    callback, callback_vec, ext_contract, init, metadata, near_bindgen, result_serializer,
    serializer, PanicOnDefault,
};

pub mod collections;
mod environment;
pub use environment::env;

mod promise;
pub use promise::{Promise, PromiseOrValue};

mod metadata;
pub use metadata::{Metadata, MethodMetadata};

pub mod json_types;

#[cfg(not(feature = "with_vm"))]
mod types;
#[cfg(not(feature = "with_vm"))]
pub use types::*;

#[cfg(feature = "with_vm")]
pub use environment::mocked_blockchain::MockedBlockchain;
#[cfg(feature = "with_vm")]
pub use near_runtime_fees::RuntimeFeesConfig;
#[cfg(feature = "with_vm")]
pub use near_vm_logic::types::*;
#[cfg(feature = "with_vm")]
pub use near_vm_logic::VMConfig;
#[cfg(feature = "with_vm")]
pub use near_vm_logic::VMContext;

pub mod utils;
pub use crate::utils::*;

pub use environment::blockchain_interface::BlockchainInterface;

#[cfg(feature = "with_vm")]
pub mod test_utils;

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
