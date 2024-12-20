//! # `near-sdk`
//!
//! `near-sdk` is a Rust toolkit for developing smart contracts on the [NEAR blockchain](https://near.org).  
//! It provides abstractions, macros, and utilities to make building robust and secure contracts easy.
//! More information on how to develop smart contracts can be found in the [NEAR documentation](https://docs.near.org/build/smart-contracts/what-is).
//! With near-sdk you can create DeFi applications, NFTs and marketplaces, DAOs, gaming and metaverse apps, and much more.
//!
//! ## Features
//!
//! - **State Management:** Simplified handling of contract state with serialization via [Borsh](https://borsh.io) or JSON.
//! - **Initialization methods** We can define an initialization method that can be used to initialize the state of the contract. #\[init\] macro verifies that the contract has not been initialized yet (the contract state doesn't exist) and will panic otherwise.
//! - **Payable methods** We can allow methods to accept token transfer together with the function call with #\[payable\] macro.
//! - **Private methods** #\[private\] macro makes it possible to define private methods that can't be called from the outside of the contract.
//! - **Cross-Contract Calls:** Support for asynchronous interactions between contracts.
//! - **Unit Testing:** Built-in support for testing contracts in a Rust environment.
//! - **WASM Compilation:** Compile Rust code to WebAssembly (WASM) for execution on the NEAR runtime.
//!
//! ## Quick Start
//!
//! Add `near-sdk` to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! near-sdk = "5.6.0"
//! ```
//!
//! ### Example: Counter Smart Contract
//!
//! Below is an example of a simple counter contract that increments and retrieves a value:
//!
//! ```rust
//! use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
//! use near_sdk::{env, near_bindgen};
//!
//! #[near_bindgen]
//! #[derive(Default, BorshDeserialize, BorshSerialize)]
//! pub struct Counter {
//!     value: i32,
//! }
//!
//! #[near_bindgen]
//! impl Counter {
//!     /// Increment the counter by one.
//!     pub fn increment(&mut self) {
//!         self.value += 1;
//!         env::log_str(&format!("Counter incremented to: {}", self.value));
//!     }
//!
//!     /// Get the current value of the counter.
//!     pub fn get(&self) -> i32 {
//!         self.value
//!     }
//! }
//! ```
//!
//! ### Compiling to WASM
//!
//! Install cargo near in case if you don't have it:
//! ```bash
//! cargo install --locked cargo-near
//! ```
//!
//! Build your contract for the NEAR blockchain:
//!
//! ```bash
//! cargo near build
//! ```
//!
//! ### Running Unit Tests
//!
//! Use the following testing setup:
//!
//! ```rust
//! #[cfg(test)]
//! mod tests {
//!     use super::*;
//!
//!     #[test]
//!     fn increment_works() {
//!         let mut counter = Counter::default();
//!         counter.increment();
//!         assert_eq!(counter.get(), 1);
//!     }
//! }
//! ```
//!
//! Run tests using:
//! ```bash
//! cargo test
//! ```

//* Clippy is giving false positive warnings for this in 1.57 version. Remove this if fixed.
//* https://github.com/rust-lang/rust-clippy/issues/8091
#![allow(clippy::redundant_closure)]
// We want to enable all clippy lints, but some of them generate false positives.
#![allow(clippy::missing_const_for_fn, clippy::redundant_pub_crate)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::needless_lifetimes)]

#[cfg(test)]
extern crate quickcheck;

pub use near_sdk_macros::{
    ext_contract, near, near_bindgen, BorshStorageKey, EventMetadata, FunctionError, NearSchema,
    PanicOnDefault,
};

pub mod store;

#[cfg(feature = "legacy")]
pub mod collections;
mod environment;
pub use environment::env;

#[cfg(feature = "unstable")]
pub use near_sys as sys;

mod promise;
pub use promise::{Allowance, Promise, PromiseOrValue};

// Private types just used within macro generation, not stable to be used.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

pub mod json_types;

mod types;
pub use crate::types::*;

#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub use environment::mock;
#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub use environment::mock::test_vm_config;
#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
// Re-export to avoid breakages
pub use environment::mock::MockedBlockchain;
#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub use test_utils::context::VMContext;

pub mod utils;
pub use crate::utils::storage_key_impl::IntoStorageKey;
pub use crate::utils::*;

#[cfg(feature = "__macro-docs")]
pub mod near;

#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub mod test_utils;

// Set up global allocator by default if custom-allocator feature is not set in wasm32 architecture.
#[cfg(all(feature = "wee_alloc", target_arch = "wasm32"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Exporting common crates

pub use base64;
pub use borsh;
pub use bs58;
#[cfg(feature = "abi")]
pub use schemars;
pub use serde;
pub use serde_json;
