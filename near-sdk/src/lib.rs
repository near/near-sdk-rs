//* Clippy is giving false positive warnings for this in 1.57 version. Remove this if fixed.
//* https://github.com/rust-lang/rust-clippy/issues/8091
#![allow(clippy::redundant_closure)]

#[cfg(test)]
extern crate quickcheck;

#[cfg(all(feature = "unstable", feature = "abi"))]
pub use near_sdk_macros::NearSchema;
pub use near_sdk_macros::{
    ext_contract, near_bindgen, BorshStorageKey, EventMetadata, FunctionError, PanicOnDefault,
};

pub mod store;

#[cfg(feature = "legacy")]
pub mod collections;
mod environment;
pub use environment::env;

#[cfg(feature = "unstable")]
pub use near_sys as sys;

mod promise;
pub use promise::{Promise, PromiseOrValue};

// Private types just used within macro generation, not stable to be used.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

pub mod json_types;

mod types;
pub use crate::types::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub use environment::mock;
#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
// Re-export to avoid breakages
pub use environment::mock::MockedBlockchain;
#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub use near_vm_logic::VMConfig;
#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub use test_utils::context::VMContext;

pub mod utils;
pub use crate::utils::storage_key_impl::IntoStorageKey;
pub use crate::utils::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub mod test_utils;

// Set up global allocator by default if custom-allocator feature is not set in wasm32 architecture.
#[cfg(all(feature = "wee_alloc", target_arch = "wasm32"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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
