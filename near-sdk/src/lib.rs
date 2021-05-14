#[cfg(test)]
extern crate quickcheck;

pub use near_sdk_macros::{
    callback, callback_vec, ext_contract, init, metadata, near_bindgen, result_serializer,
    serializer, BorshStorageKey, PanicOnDefault,
};

pub mod collections;
mod environment;
pub use environment::{env, sys};

mod promise;
pub use promise::{Promise, PromiseOrValue};

mod metadata;
pub use metadata::{Metadata, MethodMetadata};

pub mod json_types;

mod types;
pub use crate::types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use environment::mocked_blockchain::MockedBlockchain;
#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::VMConfig;
#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::VMContext;

pub mod utils;
pub use crate::utils::storage_key_impl::*;
pub use crate::utils::*;

#[cfg(not(target_arch = "wasm32"))]
pub use environment::blockchain_interface::BlockchainInterface;

#[cfg(not(target_arch = "wasm32"))]
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
