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
//! ### Example: Counter Smart Contract. For more information, see the [macro@near] documentation.
//!
//! Below is an example of a simple counter contract that increments and retrieves a value:
//!
//! ```rust
//! use near_sdk::{env, near};
//!
//! #[near(contract_state)]
//! #[derive(Default)]
//! pub struct Counter {
//!     value: i32,
//! }
//!
//! #[near]
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

/// This attribute macro is used on a struct/enum and its implementations
/// to generate the necessary code to expose `pub` methods from the contract as well
/// as generating the glue code to be a valid NEAR contract.
///
/// The macro is a syntactic sugar for [macro@near_bindgen] and expands to the [macro@near_bindgen] macro invocations.
///
/// ## Example
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers=[borsh, json])]
/// struct MyStruct {
///    pub name: String,
/// }
/// ```
///
/// This macro will generate code to load and deserialize state if the `self` parameter is included
/// as well as saving it back to state if `&mut self` is used.
///
/// # Parameter and result serialization
/// If the macro is used with Impl section, for parameter serialization, this macro will generate a struct with all of the parameters as
/// fields and derive deserialization for it. By default this will be JSON deserialized with `serde`
/// but can be overwritten by using `#[serializer(borsh)]`:
/// ```rust
/// use near_sdk::near;
///
/// # #[near(contract_state)]
/// # struct MyContract {
/// #   pub name: String,
/// # }
///
/// #[near]
/// impl MyContract {
///    #[result_serializer(borsh)]
///    pub fn borsh_parameters(&self, #[serializer(borsh)] a: String, #[serializer(borsh)] b: String) -> String {
///        format!("{} {}", a, b)
///    }
/// }
/// ```
///
/// `#[near]` will also handle serializing and setting the return value of the
/// function execution based on what type is returned by the function. By default, this will be
/// done through `serde` serialized as JSON, but this can be overridden using
/// `#[result_serializer(borsh)]`:
/// ```rust
/// use near_sdk::near;
///
/// # #[near(contract_state)]
/// # struct MyContract {
/// #   pub name: String,
/// # }
///
/// #[near]
/// impl MyContract {
///    #[result_serializer(borsh)]
///    pub fn borsh_parameters(&self) -> String {
///        self.name.clone()
///    }
/// }
/// ```
///
/// # Usage for enum / struct
///
/// If the macro is used with struct or enum, it will make the struct or enum serializable with either
/// Borsh or Json depending on serializers passed. Use `#[near(serializers=[borsh])]` to make it serializable with Borsh.
/// Or use `#[near(serializers=[json])]` to make it serializable with Json. By default, borsh is used.
/// You can also specify both and none. BorshSchema or JsonSchema are always generated if respective serializer is toggled on.
///
/// If you want the struct/enum to be a contract state, you can pass in the contract_state argument.
///
/// ## Example
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state)]
/// pub struct Contract {
///    data: i8,
/// }
///
/// #[near]
/// impl Contract {
///     pub fn some_function(&self) {}
/// }
/// ```
///
/// # Events Standard:
///
/// By passing `event_json` as an argument `near_bindgen` will generate the relevant code to format events
/// according to [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md)
///
/// For parameter serialization, this macro will generate a wrapper struct to include the NEP-297 standard fields `standard` and `version
/// as well as include serialization reformatting to include the `event` and `data` fields automatically.
/// The `standard` and `version` values must be included in the enum and variant declaration (see example below).
/// By default this will be JSON deserialized with `serde`
///
/// The version is required to allow backward compatibility. The older back-end will use the version field to determine if the event is supported.
///
/// ## Examples
///
/// ```rust
/// use near_sdk::{near, AccountId};
///
/// # #[near(contract_state)]
/// # pub struct Contract {
/// #    data: i8,
/// # }
///
///
/// #[near(event_json(standard = "nepXXX"))]
/// pub enum MyEvents {
///    #[event_version("1.0.0")]
///    Swap { token_in: AccountId, token_out: AccountId, amount_in: u128, amount_out: u128 },
///
///    #[event_version("2.0.0")]
///    StringEvent(String),
///
///    #[event_version("3.0.0")]
///    EmptyEvent
/// }
///
/// #[near]
/// impl Contract {
///     pub fn some_function(&self) {
///         MyEvents::StringEvent(
///             String::from("some_string")
///         ).emit();
///     }
///
/// }
/// ```
///
/// # Contract Source Metadata Standard:
///
/// By using `contract_metadata` as an argument `near` will populate the contract metadata
/// according to [`NEP-330`](<https://github.com/near/NEPs/blob/master/neps/nep-0330.md>) standard. This still applies even when `#[near]` is used without
/// any arguments.
///
/// All fields(version, link) are optional and will be populated with defaults from the Cargo.toml file if not specified.
/// The `standard` will be populated with `nep330` by default.
///
/// Any additional standards can be added and should be specified using the `standard` sub-attribute.
///
/// The `contract_source_metadata()` view function will be added and can be used to retrieve the source metadata.
/// Also, the source metadata will be stored as a constant, `CONTRACT_SOURCE_METADATA`, in the contract code.
///
/// ## Examples
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_metadata(
///     version = "39f2d2646f2f60e18ab53337501370dc02a5661c",
///     link = "https://github.com/near-examples/nft-tutorial",
///     standard(standard = "nep171", version = "1.0.0"),
///     standard(standard = "nep177", version = "2.0.0"),
/// ))]
/// struct Contract {}
/// ```
pub use near_sdk_macros::near;

/// `ext_contract` takes a Rust Trait and converts it to a module with static methods.
/// Each of these static methods takes positional arguments defined by the Trait,
/// then the receiver_id, the attached deposit and the amount of gas and returns a new Promise.
///
/// ## Examples
///
/// ```rust
/// use near_sdk::{AccountId,ext_contract, near, Promise, Gas};
///
/// #[near(contract_state)]
/// struct Contract {
///     calculator_account: AccountId,
/// }
///
/// #[ext_contract(ext_calculator)]
/// trait Calculator {
///     fn mult(&self, a: u64, b: u64) -> u128;
///     fn sum(&self, a: u128, b: u128) -> u128;
/// }
///
/// #[near]
/// impl Contract {
///    pub fn multiply_by_five(&mut self, number: u64) -> Promise {
///        ext_calculator::ext(self.calculator_account.clone())
///            .with_static_gas(Gas::from_tgas(5))
///            .mult(number, 5)
///    }
/// }
///
/// ```
///
/// See more information about role of ext_contract in [NEAR documentation](https://docs.near.org/build/smart-contracts/anatomy/crosscontract)
pub use near_sdk_macros::ext_contract;

/// This macro is deprecated. Use [macro@near] instead. The difference between `#[near]` and `#[near_bindgen]` is that
/// with `#[near_bindgen]` you have to manually add boilerplate code for structs and enums so that they become Json- and Borsh-serializable:
/// ```rust
/// use near_sdk::{near_bindgen, NearSchema, borsh::{BorshSerialize, BorshDeserialize}};
///
/// #[near_bindgen]
/// #[derive(BorshSerialize, BorshDeserialize, NearSchema)]
/// #[borsh(crate = "near_sdk::borsh")]
/// struct MyStruct {
///    pub name: String,
/// }
/// ```
/// Instead of:
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers=[borsh])]
/// struct MyStruct {
///     pub name: String,
/// }
/// ```
pub use near_sdk_macros::near_bindgen;

/// `BorshStorageKey` generates implementation for [BorshIntoStorageKey](crate::__private::BorshIntoStorageKey) trait.
/// It allows the type to be passed as a unique prefix for persistent collections.
/// The type should also implement or derive [BorshSerialize](borsh::BorshSerialize) trait.
///
/// More information about storage keys in [NEAR documentation](https://docs.near.org/build/smart-contracts/anatomy/storage)
/// ## Example
/// ```rust
/// use near_sdk::{BorshStorageKey, collections::Vector, near};
///
/// #[near(serializers=[borsh])]
/// #[derive(BorshStorageKey)]
/// pub enum StorageKey {
///     Messages,
/// }
///
/// // Define the contract structure
/// #[near(contract_state)]
/// pub struct Contract {
///     messages: Vector<String>
/// }
///
/// // Define the default, which automatically initializes the contract
/// impl Default for Contract {
///     fn default() -> Self {
///         Self {
///             messages: Vector::new(StorageKey::Messages)
///         }
///     }
/// }
/// ```
pub use near_sdk_macros::BorshStorageKey;

/// `PanicOnDefault` generates implementation for `Default` trait that panics with the following
/// message `The contract is not initialized` when `default()` is called.
/// This is a helpful macro in case the contract is required to be initialized with either `init` or
/// `init(rust_state)`
///
/// ## Example
/// ```rust
/// use near_sdk::{PanicOnDefault, near};
///
/// #[near(contract_state)]
/// #[derive(PanicOnDefault)]
/// pub struct Contract {
///     pub name: String,
/// }
/// ```
pub use near_sdk_macros::PanicOnDefault;

/// NOTE: This is an internal implementation for `#[near_bindgen(events(standard = ...))]` attribute.
/// Please use [macro@near] instead.
///
/// This derive macro is used to inject the necessary wrapper and logic to auto format
/// standard event logs and generate the `emit` function, and event version.
///
/// The macro is not for public use.
pub use near_sdk_macros::EventMetadata;

/// `NearSchema` is a derive macro that generates `BorshSchema` and / or `JsonSchema` implementations.
/// Use `#[abi(json)]` attribute to generate code for `JsonSchema`. And `#[abi(borsh)]` for `BorshSchema`.
/// You can use both and none as well.
/// ## Example
/// ```rust
/// use near_sdk_macros::NearSchema;
///
/// #[derive(NearSchema)]
/// #[abi(borsh)]
/// struct Value {
///    field: String,
/// }
/// ```
/// In this example, BorshSchema will be generated for `Value` struct.
pub use near_sdk_macros::NearSchema;

/// `FunctionError` generates implementation for `near_sdk::FunctionError` trait.
/// It allows contract runtime to panic with the type using its `ToString` implementation
/// as the message.
/// ## Example
/// ```rust
/// use near_sdk::{FunctionError, near};
///
/// #[derive(FunctionError)]
/// pub enum MyError {
///     Error,
/// }
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             MyError::Error => write!(f, "Error"),
///         }
///     }
/// }
///
/// #[near(contract_state)]
/// pub struct Contract {}
///
/// #[near]
/// impl Contract {
///     #[handle_result]
///     pub fn some_function(&self) -> Result<(), MyError> {
///         Err(MyError::Error)
///     }
/// }
/// ```
pub use near_sdk_macros::FunctionError;

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
