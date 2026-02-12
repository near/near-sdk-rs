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
// Allow test attribute in doctest since it's showing example testing code
#![allow(clippy::test_attr_in_doctest)]
//!
//! ```toml
//! [dependencies]
//! near-sdk = "5.17.0"
//! ```
//!
//! ### Example: Counter Smart Contract. For more information, see the [**near** macro](near) documentation.
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
//! ### Cargo NEAR Extension
//!
//! [`cargo-near`](https://github.com/near/cargo-near) is a handy command line
//! extension to `cargo`, which guides you through the common tasks of
//! creating, building, and deploying smart contracts.
//!
//! Follow the [installation instructions](https://github.com/near/cargo-near?tab=readme-ov-file#installation) on cargo-near README.
//!
//! Or compile it and install it from the source code:
//!
//! ```bash
//! cargo install --locked cargo-near
//! ```
//!
//! ### Create New NEAR Smart Contract
//!
//! `cargo-near` can be used to start a new project with an example smart contract, unit tests,
//! integration tests, and continuous integration preconfigured for you.
//!
//! ```bash
//! cargo near new
//! ```
//!
//! ### Compiling to WASM
//!
//! `cargo-near` builds a NEAR smart contract along with its [ABI](https://github.com/near/abi) (while in the directory containing contract's Cargo.toml):
//!
//! ```bash
//! cargo near build
//! ```
//!
//! If you have problems/errors with schema/ABI during build that you cannot figure out quick, you can skip/circumvent them with:
//!
//! ```bash
//! cargo near build non-reproducible-wasm --no-abi
//! ```
//!
//! And return to figuring how to resolve problems with generating ABI of your contract later.
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

#![cfg_attr(docsrs, feature(doc_cfg))]
// Clippy is giving false positive warnings for this in 1.57 version. Remove this if fixed.
// https://github.com/rust-lang/rust-clippy/issues/8091
#![allow(clippy::redundant_closure)]
// We want to enable all clippy lints, but some of them generate false positives.
#![allow(clippy::missing_const_for_fn, clippy::redundant_pub_crate)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::needless_lifetimes)]

#[cfg(test)]
extern crate quickcheck;

#[cfg(not(any(
    test,
    doctest,
    clippy,
    target_family = "wasm",
    feature = "unit-testing",
    feature = "non-contract-usage",
    feature = "__abi-generate"
)))]
compile_error!(
    r#"1. 🔨️  Use `cargo near build` instead of `cargo build` to compile your contract
💡  Install cargo-near from https://github.com/near/cargo-near

2. ✅ Use `cargo check --target wasm32-unknown-unknown` instead of `cargo check` to error-check your contract

3. ⚙️ Only following cfg-s are considered VALID for `near-sdk`:
  - `#[cfg(target_family = "wasm")]`
  - `#[cfg(feature = "non-contract-usage")]` (intended for use of `near-sdk` in non-contract environment)
  - `#[cfg(feature = "unit-testing")]` (intended for use of `near-sdk` as one of `[dev-dependencies]`)
  - `#[cfg(feature = "__abi-generate")`
  - `#[cfg(test)]`
  - `#[cfg(doctest)]`
  - `#[cfg(clippy)]`
⚠️ a cfg, which is not one of the above, results in CURRENT compilation error to be emitted.
"#
);

/// This attribute macro is used on a struct/enum and its implementations
/// to generate the necessary code to expose `pub` methods from the contract as well
/// as generating the glue code to be a valid NEAR contract.
///
/// The macro is a syntactic sugar for [**near_bindgen**](near_bindgen) and expands to the [**near_bindgen**](near_bindgen) macro invocations.
/// Both of them share the same attributes, except for those that are explicitly marked as specific to the [**near**](near) macro. ([1](near#nearcontract_state-annotates-structsenums), [2](near#nearserializers-annotates-structsenums))
///
/// # Attributes
///
/// ## `#[near(contract_state)]` (annotates structs/enums)
///
/// The attribute prepares a struct/enum to be a contract state. Only one contract state is allowed per crate.
///
/// Custom storage key can be set via `#[near(contract_state(key = b"CUSTOM"))]`.
///
/// A contract type is usually acompanied by an `impl` block, annotated with [`#[near]`](near#near-annotates-impl-blocks).
///
/// This attribute is also required to make the [`#[near(contract_metadata(...))]`](near#nearcontract_metadata-annotates-structsenums) attribute work.
///
/// `contract_state` is specific to the [near] macro only, not available for [near_bindgen].
///
/// ### Basic example
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state)]
/// pub struct Contract {
///     greeting: String,
/// }
/// ```
/// which usually comes paired with at least one **impl** block for the contract type,
/// annotated with a plain `#[near]` attribute:
///
/// ### Using SDK collections for storage
///
/// If contract state becomes large, collections from following modules can be used:
///
/// #### [`store`] module:
///
/// ```rust
/// # use near_sdk_macros::near;
/// use near_sdk::store::IterableMap;
///
/// #[near(contract_state)]
/// pub struct StatusMessage {
///    records: IterableMap<String, String>,
/// }
/// ```
///
/// * list of [**host functions**](store#calls-to-host-functions-used-in-implementation) used for [`store`] implementation
/// * **FAQ**: mutating state of collections from [`store`] module is only finally persisted on running [`Drop`/`flush`](store#faq-collections-of-this-module-only-persist-on-drop-and-flush)
///
/// #### [`collections`] module:
///
/// ```rust
/// # use near_sdk_macros::near;
/// use near_sdk::collections::LookupMap;
///
/// #[near(contract_state)]
/// pub struct StatusMessage {
///    records: LookupMap<String, String>,
/// }
/// ```
///
/// * list of [**host functions**](collections#calls-to-host-functions-used-in-implementation) used for [`collections`] implementation
///
/// ### Reference to [Implementation of `#[near(contract_state)]` attribute](near#implementation-of-nearcontract_state-attribute-and-host-functions-calls-used) (How does it work?)
///
/// ### Auto-generated [`<ContractType>Ext`](near#contracttypeext-struct-auto-generated-for-cross-contract-calls) struct
///
/// This attribute also generates the `<ContractType>Ext` struct definition for cross-contract calls.
/// See [`<ContractType>Ext` documentation](near#contracttypeext-struct-auto-generated-for-cross-contract-calls) for details.
///
/// ## `#[near]` (annotates impl blocks)
///
/// This macro is used to define the code for view-only and mutating methods for contract types,
/// annotated by [`#[near(contract_state)]`](near#nearcontract_state-annotates-structsenums).
///
/// ### Basic example
/// ```rust
/// use near_sdk::{near, log};
///
/// # #[near(contract_state)]
/// # pub struct Contract {
/// #     greeting: String,
/// # }
/// #[near]
/// impl Contract {
///     // view method
///     pub fn get_greeting(&self) -> String {
///         self.greeting.clone()
///     }
///
///     // mutating method
///     pub fn set_greeting(&mut self, greeting: String) {
///         log!("Saving greeting: {greeting}");
///         self.greeting = greeting;
///     }
/// }
/// ```
///
/// ### Reference to [Implementation of `#[near]` macro](near#implementation-of-near-macro-and-host-functions-calls-used) (How does it work?)
///
/// ### Auto-generated method wrappers on [`<ContractType>Ext`](near#contracttypeext-struct-auto-generated-for-cross-contract-calls)
///
/// This macro also generates method wrappers on the `<ContractType>Ext` struct for each public method,
/// enabling cross-contract calls.
/// See [`<ContractType>Ext` documentation](near#contracttypeext-struct-auto-generated-for-cross-contract-calls) for details.
///
/// ## `<ContractType>Ext` struct (auto-generated for cross-contract calls)
///
/// When you annotate a struct with [`#[near(contract_state)]`](near#nearcontract_state-annotates-structsenums)
/// and define methods in an [`#[near]` impl block](near#near-annotates-impl-blocks), the macro automatically
/// generates a companion struct called `<ContractType>Ext` (e.g., `ContractExt` for a contract named `Contract`).
///
/// This struct provides a **builder pattern API for making cross-contract calls** to your contract's methods,
/// returning a [`Promise`] that can be chained with other promises.
///
/// ### Generated structure
///
/// For a contract like:
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state)]
/// pub struct CrossContract {
///     greeting: String,
/// }
///
/// #[near]
/// impl CrossContract {
///     pub fn method_one(&self, n: u32) -> u32 { n }
///     pub fn method_two(&mut self, message: String) { }
/// }
/// ```
///
/// The macro generates (approximately):
///
/// ```rust,ignore
/// #[must_use]
/// pub struct CrossContractExt {
///     pub(crate) promise_or_create_on: PromiseOrValue<AccountId>,
///     pub(crate) deposit: NearToken,
///     pub(crate) static_gas: Gas,
///     pub(crate) gas_weight: GasWeight,
/// }
///
/// impl CrossContract {
///     /// API for calling this contract's functions in a subsequent execution.
///     pub fn ext(account_id: AccountId) -> CrossContractExt { /* ... */ }
///
///     /// API for calling this contract's functions as a callback on a promise.
///     pub fn ext_on(promise: Promise) -> CrossContractExt { /* ... */ }
/// }
///
/// impl CrossContractExt {
///     /// Attach NEAR tokens to the cross-contract call.
///     pub fn with_attached_deposit(mut self, amount: NearToken) -> Self { /* ... */ }
///
///     /// Specify the amount of static gas to attach to this call.
///     pub fn with_static_gas(mut self, static_gas: Gas) -> Self { /* ... */ }
///
///     /// Specify the weight for distributing unused gas to this call.
///     pub fn with_unused_gas_weight(mut self, gas_weight: u64) -> Self { /* ... */ }
///
///     // Methods mirroring contract methods:
///     pub fn method_one(self, n: u32) -> Promise { /* ... */ }
///     pub fn method_two(self, message: String) -> Promise { /* ... */ }
/// }
/// ```
///
/// ### What gets generated where
///
/// - **`#[near(contract_state)]`** generates the `<ContractType>Ext` struct definition
///   with its fields and the `ext()` / `ext_on()` constructor methods.
/// - **`#[near]` on impl blocks** generates method wrappers on `<ContractType>Ext`
///   that mirror each public method in the impl block, returning a [`Promise`].
///
/// ### Usage example: Cross-contract calls
///
/// ```rust
/// use near_sdk::{near, env, Promise};
///
/// # #[near(contract_state)]
/// # pub struct Contract {
/// #     other_contract_id: near_sdk::AccountId,
/// # }
///
/// #[near]
/// impl Contract {
///     pub fn some_method(&self) -> u32 { 42 }
///
///     pub fn call_other_contract(&self) -> Promise {
///         // Call another contract's method using the Ext struct
///         Self::ext(self.other_contract_id.clone())
///             .with_attached_deposit(near_sdk::NearToken::from_near(1))
///             .with_static_gas(near_sdk::Gas::from_tgas(5))
///             .some_method()
///     }
///
///     pub fn call_with_callback(&self) -> Promise {
///         // Chain multiple calls: call self, then callback
///         Self::ext(env::current_account_id())
///             .some_method()
///             .then(
///                 Self::ext(env::current_account_id())
///                     .callback_method()
///             )
///     }
///
///     #[private]
///     pub fn callback_method(&self, #[callback_unwrap] result: u32) {
///         // Handle the result from the previous call
///     }
/// }
/// ```
///
/// ### Discovering method signatures
///
/// To explore all available methods on your `<ContractType>Ext` struct, run:
///
/// ```bash,ignore
/// cargo doc --lib --open
/// ```
///
/// This generates documentation for your contract, including the auto-generated
/// `<ContractType>Ext` struct with all its methods and their signatures.
///
/// ### See also
///
/// - [`Promise`] - The type returned by `<ContractType>Ext` methods
/// - [`#[callback_unwrap]`](near#callback_unwrap-annotates-function-arguments) - For handling results from cross-contract calls
/// - [`#[private]`](near#private-annotates-methods-of-a-type-in-its-impl-block) - For restricting callback methods
/// - [NEAR Cross-Contract Calls Documentation](https://docs.near.org/build/smart-contracts/anatomy/crosscontract)
///
/// ## `#[near(serializers=[...])` (annotates structs/enums)
///
/// The attribute makes the struct or enum serializable with either json or borsh. By default, borsh is used.
///
/// `serializers` is specific to the [near] macro only, not available for [near_bindgen].
///
/// ### Make struct/enum serializable with borsh
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers=[borsh])]
/// pub enum MyEnum {
///     Variant1,
/// }
///
/// #[near(serializers=[borsh])]
/// pub struct MyStruct {
///     pub name: String,
/// }
///
///
/// // Since [borsh] is the default value, you can simply skip serializers:
///
/// #[near]
/// pub enum MyEnum2 {
///     Variant1,
/// }
///
/// #[near]
/// pub struct MyStruct2 {
///     pub name: String,
/// }
/// ```
///
/// ### Make struct/enum serializable with json
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers=[json])]
/// pub enum MyEnum {
///     Variant1,
/// }
///
/// #[near(serializers=[json])]
/// pub struct MyStruct {
///     pub name: String,
/// }
/// ```
///
/// ### Make struct/enum serializable with both borsh and json
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers=[borsh, json])]
/// pub enum MyEnum {
///     Variant1,
/// }
///
/// #[near(serializers=[borsh, json])]
/// pub struct MyStruct {
///     pub name: String,
/// }
/// ```
///
/// ### Customize `borsh` serializer
///
/// The `#[near(serializers = [borsh(...)])]` macro allows you to pass [configuration parameters to the `borsh` serializer](https://docs.rs/borsh/latest/borsh/derive.BorshSerialize.html#attributes).
/// This is useful for customizing borsh serialization parameters since, unlike serde, borsh macros do not support repetitive attributes.
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers = [borsh(use_discriminant = true)])]
/// pub enum MyEnum {
///     Variant1,
///     Variant2,
/// }
/// ```
///
/// ### Customize `json` serializer
///
/// The `#[near(serializers = [json])]` macro does not support passing configuration parameters to the `json` serializer.
/// Yet, you can just use [`#[serde(...)]` attributes](https://serde.rs/attributes.html) as if `#[derive(Serialize, Deserialize)]` is added to the struct (which is what actually happens under the hood of `#[near(serializers = [json])]` implementation).
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(serializers = [json])]
/// #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// pub enum MyEnum {
///     Variant1,
///     #[serde(alias = "VARIANT_2")]
///     Variant2,
/// }
/// ```
///
/// You can also use [`#[serde_as(as = "...")]` attributes](https://docs.rs/serde_with/latest/serde_with/attr.serde_as.html)
/// as if `#[serde_as]` is added to the type (which is what actually happens under the hood of `#[near(serializers = [json])]` implementation).
///
/// Note: When using the `abi` feature with base64/hex encoding, prefer the SDK's [`json_types`](crate::json_types)
/// like [`Base64VecU8`](crate::json_types::Base64VecU8), which have full JSON Schema support.
///
/// For hex encoding with ABI support, you can use `#[serde_as(as = "serde_with::hex::Hex")]` by enabling
/// the `serde_with/schemars_1` feature in your `Cargo.toml` (this requires upgrading to schemars 1.x).
///
/// ```
/// # use std::collections::BTreeMap;
/// use near_sdk::{
///     near,
///     serde_json::json,
///     serde_with::{json::JsonString, DisplayFromStr},
///     json_types::Base64VecU8,
/// };
///
/// #[near(serializers = [json])]
/// pub struct MyStruct {
///     #[serde_as(as = "DisplayFromStr")]
///     pub amount: u128,
///
///     pub base64_bytes: Base64VecU8,
///
///     #[serde_as(as = "BTreeMap<DisplayFromStr, Vec<DisplayFromStr>>")]
///     pub collection: BTreeMap<u128, Vec<u128>>,
///
///     #[serde_as(as = "JsonString")]
///     pub json_string: serde_json::Value,
/// }
/// # fn main() {
/// #     assert_eq!(
/// #         serde_json::to_value(&MyStruct {
/// #             amount: u128::MAX,
/// #             base64_bytes: Base64VecU8::from(vec![1, 2, 3]),
/// #             collection: [(u128::MAX, vec![100, 200, u128::MAX])].into(),
/// #             json_string: json!({"key": "value"}),
/// #         })
/// #         .unwrap(),
/// #         json!({
/// #             "amount": "340282366920938463463374607431768211455",
/// #             "base64_bytes": "AQID",
/// #             "collection": {
/// #                 "340282366920938463463374607431768211455": ["100", "200", "340282366920938463463374607431768211455"],
/// #             },
/// #             "json_string": "{\"key\":\"value\"}",
/// #         })
/// #     );
/// # }
/// ```
///
/// ## `#[serializer(...)]` (annotates function arguments)
///
/// The attribute makes the function argument deserializable from [`Vec`]<[`u8`]> with either json or borsh. By default, json is used.
/// Please, note that all the arguments of the function should be using the same deserializer.
///
/// NOTE: a more correct name for the attribute would be `argument_deserializer`, but it's `serializer` for historic reasons.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::near;
///# #[near(contract_state)]
///# pub struct Contract {}
///
/// #[near]
/// impl Contract {
///     pub fn borsh_arguments(&self, #[serializer(borsh)] a: String, #[serializer(borsh)] b: String) {}
/// }
/// ```
///
/// ### Implementation of `#[serializer(...)]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// using the attribute allows to replace default [`serde_json::from_slice`] with [`borsh::from_slice`].
///
/// A bit more thoroughly the effect of the attribute is described in (step **3.1**, [`#[near]` on mutating method](near#for-above-mutating-method-near-macro-defines-the-following-function)).
///
/// ## `#[init]` (annotates methods of a type in its `impl` block)
///
/// Contract initialization method annotation. More details can be found [here](https://docs.near.org/build/smart-contracts/anatomy/storage#initializing-the-state)
///
/// By default, the `Default::default()` implementation of a contract will be used to initialize a contract.
/// There can be a custom initialization function which takes parameters or performs custom logic with the following `#[init]` annotation.
///
/// You can provide several initialization functions.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::{log, near};
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Counter {
///     value: u64,
/// }
///
/// #[near]
/// impl Counter {
///     #[init]
///     pub fn new(value: u64) -> Self {
///         log!("Custom counter initialization!");
///         Self { value }
///     }
/// }
/// ```
///
/// ### Implementation of `#[init]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// For a method annotated with `#[init]`:
///
/// 1. Before invoking the constructor, the macro checks if the contract state already exists by calling
///    [`state::ContractState::state_exists`](crate::state::ContractState::state_exists), which internally uses
///    [`env::storage_has_key`] to check for the state key
/// 2. If the state already exists, [`env::panic_str`] host function is called with the message
///    `"The contract has already been initialized"`
/// 3. Otherwise, the constructor method is called to create the contract instance
/// 4. The newly created contract state is written using [`env::state_write`] host function
///
/// The `#[init(ignore_state)]` variant skips the state existence check in step 1-2, allowing
/// re-initialization of the contract.
///
/// ## `#[payable]` (annotates methods of a type in its `impl` block)
///
/// Specifies that the method can accept NEAR tokens. More details can be found [here](https://docs.near.org/build/smart-contracts/anatomy/functions#payable-functions)
///
/// Methods can be annotated with `#[payable]` to allow tokens to be transferred with the method invocation. For more information, see payable methods.
///
/// To declare a function as payable, use the `#[payable]` annotation as follows:
///
/// ### Basic example
///
/// ```rust
///use near_sdk::near;
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Counter {
///     val: i8,
/// }
///
/// #[near]
/// impl Counter {
///     #[payable]
///     pub fn my_method(&mut self) {
///        //...
///     }
/// }
/// ```
///
/// ### Implementation of `#[payable]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// For methods **without** the `#[payable]` attribute, the macro generates a deposit check at the beginning
/// of the method:
///
/// 1. [`env::attached_deposit`] host function is called to get the amount of NEAR tokens attached to the call
/// 2. If the attached deposit is not zero, [`env::panic_str`] host function is called with the message
///    `"Method {method_name} doesn't accept deposit"`
///
/// When a method is annotated with `#[payable]`, this deposit check is skipped, allowing the method
/// to accept NEAR token transfers along with the function call.
///
/// ## `#[private]` (annotates methods of a type in its `impl` block)]
///
/// The attribute forbids to call the method except from within the contract.
/// This is useful for internal methods that should not be called from outside the contract.
///
/// More details can be found [here](https://docs.near.org/build/smart-contracts/anatomy/functions#private-functions)
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Counter {
///     val: u64,
/// }
///
/// #[near]
/// impl Counter {
///     #[private]
///     pub fn my_method(&mut self) {
///         // ...
///     }
/// }
/// ```
///
/// ### Implementation of `#[private]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// For methods annotated with `#[private]`, the macro generates a caller check at the beginning
/// of the method:
///
/// 1. [`env::current_account_id`] host function is called to get the contract's own account ID
/// 2. [`env::predecessor_account_id`] host function is called to get the caller's account ID
/// 3. If the caller's account ID does not match the contract's account ID, [`env::panic_str`] host function
///    is called with the message `"Method {method_name} is private"`
///
/// This ensures that only the contract itself (through cross-contract calls from its own methods)
/// can invoke the private method.
///
/// ## `#[deny_unknown_arguments]` (annotates methods of a type in its `impl` block)]
///
/// Specifies that the method call should error during deserialization if any unknown fields are present in the input.
/// This helps ensure data integrity by rejecting potentially malformed input.
///
/// Without this attribute, unknown fields are silently ignored during deserialization.
///
/// Implementation uses [`deny_unknown_fields`](https://serde.rs/container-attrs.html#deny_unknown_fields) `serde`'s attribute.
///
/// In the following example call of `my_method` with
/// ```json,ignore
/// {
///     "description": "value of description"
/// }
/// ```
/// payload works, but call of `my_method` with
///
/// ```json,ignore
/// {
///     "description": "value of description",
///     "unknown_field": "what"
/// }
/// ```
/// payload is declined with a `FunctionCallError(ExecutionError("Smart contract panicked: Failed to deserialize input from JSON."))` error.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Counter {
///     val: u64,
/// }
///
/// #[near]
/// impl Counter {
///     #[deny_unknown_arguments]
///     pub fn my_method(&mut self, description: String) {
///         // ...
///     }
/// }
/// ```
///
/// This attribute is not supposed to be used together with [`#[serializer(borsh)]`](`near#serializer-annotates-function-arguments`)
/// arguments' serializer, and assumes that default `json` is used.
///
/// If `borsh` is used on arguments, usage of `deny_unknown_arguments` on method is a no-op.
///
///
/// ## `#[result_serializer(...)]` (annotates methods of a type in its `impl` block)
///
/// The attribute defines the serializer for function return serialization.
/// Only one of `borsh` or `json` can be specified.
///
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
///    pub fn borsh_return_value(&self) -> String {
///         "hello_world".to_string()
///    }
/// }
/// ```
///
/// ### Implementation of `#[result_serializer(...)]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// using the attribute allows to replace default [`serde_json::to_vec`] with [`borsh::to_vec`].
///
/// A bit more thoroughly the effect of the attribute is described in (step **4.1**, [`#[near] on view method`](near#for-above-view-method-near-macro-defines-the-following-function)).
///
/// ## `#[handle_result]` (annotates methods of a type in its `impl` block)
///
/// Have `#[handle_result]` to Support Result types regardless of how they're referred to
/// Function marked with `#[handle_result]` should return `Result<T, E>` (where E implements [FunctionError]).
/// If you're trying to use a type alias for `Result`, try `#[handle_result(aliased)]`
///
/// ### Basic error handling with Result
///
/// ```rust
/// use near_sdk::{near, AccountId, Promise, PromiseError};
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Counter {
///     val: u64,
/// }
///
/// #[near]
/// impl Counter {
///     #[handle_result]
///     pub fn some_function2(
///         &self,
///     ) -> Result<(), &'static str> {
///         Err("error")
///     }
/// }
/// ```
///
/// ### Typed error handling
///
/// This example shows how to use error handling in a contract when the error are defined in the contract.
/// This way the contract can utilize result types and panic with the type using its [ToString] implementation
///
/// ```rust
/// use near_sdk::{near, FunctionError};
///
/// #[derive(FunctionError)]
/// pub enum MyError {
///     SomePanicError,
/// }
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             MyError::SomePanicError => write!(f, "Panic error message that would be displayed to the user"),
///         }
///     }
/// }
///# #[near(contract_state)]
///# #[derive(Default)]
///# pub struct Counter {
///#    val: u64,
///# }
///
/// #[near]
/// impl Counter {
///     #[handle_result]
///     pub fn some_function(&self) -> Result<(), MyError> {
///         if self.val == 0 {
///             return Err(MyError::SomePanicError);
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// ### Implementation of `#[handle_result]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// For methods annotated with `#[handle_result]`, the macro modifies how the return value is processed:
///
/// 1. The method is expected to return `Result<T, E>` where `E` implements [`FunctionError`]
/// 2. After the method executes, the macro checks if the result is:
///    - [`Result::Ok`]: The inner value is serialized and returned via [`env::value_return`] host function
///    - [`Result::Err`]: The error's [`FunctionError::panic`] method is called, which typically
///      calls [`env::panic_str`] host function with the error message converted via [`ToString`]
///
/// The `#[handle_result(aliased)]` variant allows using type aliases for `Result` types, enabling
/// custom result type definitions while maintaining the same behavior.
///
/// ## `#[callback_unwrap]` (annotates function arguments)
///
/// Automatically unwraps the successful result of a callback from a cross-contract call.
/// Used on parameters in callback methods that are invoked as part of a cross-contract call chain.
/// If the promise fails, the method will panic with the error message.
///
/// This attribute is commonly used with [`Promise`] or [`PromiseOrValue<T>`] as the return type of another contract method,
/// whose return value will be passed as argument to `#[callback_unwrap]`-annotated argument
///
/// ### Example with Cross-Contract Factorial:
///
/// In the example:
///   - lower level [`env::promise_create`], [`env::promise_then`] and [`env::promise_return`] are used in
///     `factorial` method to set up a callback of `factorial_mult` with result of factorial for `(n-1)`
///   - [`#[private]`](near#private-annotates-methods-of-a-type-in-its-impl-block) on `factorial_mult` is used to
///     to allow only calling `factorial_mult` from factorial contract method by `CrossContract` itself
///     and disallow for it to be called externally by users
///
/// ```rust
/// use near_sdk::{near, env, log, NearToken, Gas};
///
/// // Prepaid gas for a single (not inclusive of recursion) `factorial` call.
/// const FACTORIAL_CALL_GAS: Gas = Gas::from_tgas(20);
///
/// // Prepaid gas for a single `factorial_mult` call.
/// const FACTORIAL_MULT_CALL_GAS: Gas = Gas::from_tgas(10);
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct CrossContract {}
///
/// #[near]
/// impl CrossContract {
///     pub fn factorial(&self, n: u32) {
///         if n <= 1 {
///             env::value_return(&serde_json::to_vec(&1u32).unwrap());
///             return;
///         }
///         let account_id = env::current_account_id();
///         let prepaid_gas = env::prepaid_gas().saturating_sub(FACTORIAL_CALL_GAS);
///         let promise0 = env::promise_create(
///             account_id.clone(),
///             "factorial",
///             &serde_json::to_vec(&(n - 1,)).unwrap(),
///             NearToken::from_near(0),
///             prepaid_gas.saturating_sub(FACTORIAL_MULT_CALL_GAS),
///         );
///         let promise1 = env::promise_then(
///             promise0,
///             account_id,
///             "factorial_mult",
///             &serde_json::to_vec(&(n,)).unwrap(),
///             NearToken::from_near(0),
///             FACTORIAL_MULT_CALL_GAS,
///         );
///         env::promise_return(promise1);
///     }
///
///     #[private]
///     pub fn factorial_mult(&self, n: u32, #[callback_unwrap] factorial_n_minus_one_result: u32) -> u32 {
///         log!("Received n: {:?}", n);
///         log!("Received factorial_n_minus_one_result: {:?}", factorial_n_minus_one_result);
///
///         let result = n * factorial_n_minus_one_result;
///
///         log!("Multiplied {:?}", result.clone());
///         result
///     }
/// }
/// ```
/// which has the following lines in a `factorial`'s view call log:
///
/// ```bash,ignore
/// logs: [
///     "Received n: 5",
///     "Received factorial_n_minus_one_result: 24",
///     "Multiplied 120",
/// ],
/// ```
///
/// ### Other examples within repo:
///
/// - `Cross-Contract Factorial` again [examples/cross-contract-calls](https://github.com/near/near-sdk-rs/blob/9596835369467cac6198e8de9a4b72a38deee4a5/examples/cross-contract-calls/high-level/src/lib.rs?plain=1#L26)
///   - same example as [above](near#example-with-cross-contract-factorial), but uses [`Promise::then`] instead of [`env`](mod@env) host functions calls to set up a callback of `factorial_mult`
/// - [examples/callback-results](https://github.com/near/near-sdk-rs/blob/c2a2d36b2a83ad8fe110c3b21046064f581dc458/examples/callback-results/src/lib.rs?plain=1#L60)
///
/// ### Reference to  [Implementation of `#[callback_unwrap]` attribute](near#implementation-of-callback_unwrap-attribute-and-host-functions-calls-used)
///
/// ## `#[callback_result]` (annotates function arguments)
///
/// Similar to [`#[callback_unwrap]`](near#callback_unwrap-annotates-function-arguments), but instead of panicking on promise failure,
/// it wraps the result in a `Result<T, PromiseError>`, allowing the callback to handle both success and failure cases.
///
/// This is useful when you want to handle failed cross-contract calls gracefully instead of panicking.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::{near, PromiseError};
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Contract {}
///
/// #[near]
/// impl Contract {
///     #[private]
///     pub fn callback_method(&self, #[callback_result] result: Result<String, PromiseError>) {
///         match result {
///             Ok(value) => {
///                 // Handle successful cross-contract call
///                 near_sdk::log!("Received value: {}", value);
///             }
///             Err(_) => {
///                 // Handle failed cross-contract call
///                 near_sdk::log!("Cross-contract call failed");
///             }
///         }
///     }
/// }
/// ```
///
/// ### Implementation of `#[callback_result]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// For arguments annotated with `#[callback_result]`:
///
/// 1. Arguments are not expected to be included in the regular input deserialization
/// 2. For each `#[callback_result]` argument:
///    1. [`env::promise_result_checked`] host function is called with the corresponding index
///       (0 for the first callback argument, 1 for the second, etc.)
///    2. If successful, the data is deserialized and wrapped in [`Result::Ok`]
///    3. If the promise failed, [`Result::Err(PromiseError)`](crate::PromiseError) is returned
///       (no panic occurs, unlike `#[callback_unwrap]`)
/// 3. The resulting `Result<T, PromiseError>` is passed to the method, allowing error handling
///
/// The optional `max_bytes` parameter (e.g., `#[callback_result(max_bytes = 100)]`) limits the
/// maximum size of the callback data to prevent excessive memory usage.
///
/// ## `#[callback_vec]` (annotates function arguments)
///
/// Collects results from multiple promises into a `Vec<T>`. This is useful when you have
/// multiple cross-contract calls via [`Promise::and`] and want to process all their results.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// pub struct Contract {}
///
/// #[near]
/// impl Contract {
///     #[private]
///     pub fn aggregate_callback(&self, #[callback_vec] results: Vec<u64>) -> u64 {
///         // Sum all results from multiple cross-contract calls
///         results.iter().sum()
///     }
/// }
/// ```
///
/// ### Implementation of `#[callback_vec]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// For the argument annotated with `#[callback_vec]`:
///
/// 1. [`env::promise_results_count`] host function is called to get the total number of promise results
/// 2. For each promise result (from index 0 to count-1):
///    1. [`env::promise_result_checked`] host function is called with the index
///    2. If successful, the data is deserialized and added to the vector
///    3. If any promise failed, [`env::panic_str`] host function is called with the message
///       `"Callback computation {index} was not successful"`
/// 3. The collected `Vec<T>` is passed to the method
///
/// **Note:** Only one `#[callback_vec]` parameter is allowed per method.
///
/// The optional `max_bytes` parameter limits the maximum size of each callback result.
///
/// ## `#[near(event_json(...))]` (annotates enums)
///
/// By passing `event_json` as an argument `near` will generate the relevant code to format events
/// according to [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md)
///
/// For parameter serialization, this macro will generate a wrapper struct to include the NEP-297 standard fields `standard` and `version`
/// as well as include serialization reformatting to include the `event` and `data` fields automatically.
/// The `standard` and `version` values must be included in the enum and variant declaration (see example below).
/// By default this will be JSON deserialized with `serde`
///
/// The version is required to allow backward compatibility. The older back-end will use the version field to determine if the event is supported.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::{near, AccountId};
///
/// # #[near(contract_state)]
/// # pub struct Contract {
/// #    data: i8,
/// # }
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
/// ### Implementation of `#[near(event_json(...))]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// The `#[near(event_json(standard = "..."))]` macro transforms an enum into a NEP-297 compliant event:
///
/// 1. The macro adds `#[derive(Serialize, EventMetadata)]` to the enum
/// 2. Serde attributes are added for proper JSON serialization:
///    - `#[serde(tag = "event", content = "data")]` for the NEP-297 format
///    - `#[serde(rename_all = "snake_case")]` for event name formatting
/// 3. A constant `{EnumName}_event_standard` is generated with the standard name
/// 4. The [`EventMetadata`](crate::EventMetadata) derive macro generates:
///    - `emit()` method: Serializes the event to JSON and calls [`env::log_str`] with the format
///      `EVENT_JSON:{json}` as specified by [NEP-297](https://github.com/near/NEPs/blob/master/neps/nep-0297.md)
///    - `to_json()` method: Returns the event as a [`serde_json::Value`]
///    - `standard()`, `version()`, `event()` methods for accessing metadata
/// 5. Each variant must have `#[event_version("x.x.x")]` to specify the version
///
/// ## `#[near(contract_metadata(...))]` (annotates structs/enums)
///
/// By using `contract_metadata` as an argument `near` will populate the contract metadata
/// according to [`NEP-330`](<https://github.com/near/NEPs/blob/master/neps/nep-0330.md>) standard. This still applies even when `#[near]` is used without
/// any arguments.
///
/// All fields(version, link) are optional and will be populated with defaults from the Cargo.toml file if not specified.
/// The `standard` will be populated with `nep330` by default.
///
/// **Any additional standards can be added and should be specified using the `standard` attribute.**
///
/// The `contract_source_metadata()` view function will be added and can be used to retrieve the source metadata.
/// Also, the source metadata will be stored as a constant, `CONTRACT_SOURCE_METADATA`, in the contract code.
///
/// **Please note that the `contract_metadata` will be ignored if [`#[near(contract_state)]`](near#nearcontract_state-annotates-structsenums) is not used**.
///
/// ### Basic example
///
/// ```rust
/// use near_sdk::near;
///
/// #[near(contract_state, contract_metadata(
///     version = "39f2d2646f2f60e18ab53337501370dc02a5661c",
///     link = "https://github.com/near-examples/nft-tutorial",
///     standard(standard = "nep171", version = "1.0.0"),
///     standard(standard = "nep177", version = "2.0.0"),
/// ))]
/// struct Contract {}
/// ```
///
/// ### Implementation of `#[near(contract_metadata(...))]` attribute and **host functions** calls used
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// The `#[near(contract_metadata(...))]` attribute works in conjunction with `#[near(contract_state)]`:
///
/// 1. The metadata is extracted from the attribute arguments or defaults from `Cargo.toml`:
///    - `version`: Defaults to the `NEP330_VERSION` environment variable, or if unset, to `CARGO_PKG_VERSION`
///    - `link`: Defaults to the `NEP330_LINK` environment variable, falling back to `CARGO_PKG_REPOSITORY` if unset
///    - `standard`: Additional standards the contract implements (e.g., NEP-171, NEP-177)
/// 2. A `CONTRACT_SOURCE_METADATA` constant is generated containing the JSON-serialized metadata
/// 3. A `contract_source_metadata()` view function is generated that:
///    1. Calls [`env::setup_panic_hook`] host function
///    2. Calls [`env::value_return`] host function with the `CONTRACT_SOURCE_METADATA` bytes
///
/// This follows the [NEP-330](https://github.com/near/NEPs/blob/master/neps/nep-0330.md) standard for
/// contract source metadata, allowing tools and users to discover information about the deployed contract.
///
/// ---
///
/// ## Implementation of `#[near(contract_state)]` attribute and **host functions** calls used
///
/// This heading describes [`#[near(contract_state)]`](near#nearcontract_state-annotates-structsenums).
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// ```rust
/// # use near_sdk::near;
/// #[near(contract_state)]
/// pub struct Contract { /* .. */ }
/// ```
///
/// 1. Macro adds derived implementations of [`borsh::BorshSerialize`]/[`borsh::BorshSerialize`] for `Contract` type
/// 2. Macro defines a global `CONTRACT_SOURCE_METADATA` variable, which is a string of json serialization of [`near_contract_standards::contract_metadata::ContractSourceMetadata`](https://docs.rs/near-contract-standards/latest/near_contract_standards/contract_metadata/struct.ContractSourceMetadata.html).
/// 3. Macro defines `contract_source_metadata` function:
///     ```rust,no_run
///     #[unsafe(no_mangle)]
///     pub extern "C" fn contract_source_metadata() { /* .. */ }
///     ```
///    which
///     1. calls [`env::setup_panic_hook`] host function
///     2. calls [`env::value_return`] host function with bytes of `CONTRACT_SOURCE_METADATA` from step 2.
///
/// ##### using [cargo-expand](https://crates.io/crates/cargo-expand) to view actual macro results
///
/// The above is an approximate description of what macro performs.
///
/// Running the following in a contract's crate is a way to introspect more details of its operation:
///
/// ```bash,ignore
/// cargo expand --lib --target wasm32-unknown-unknown
/// # this has additional code generated for ABI layer
/// cargo expand --lib --features near-sdk/__abi-generate
/// ```
///
/// ---
///
/// ## Implementation of `#[near]` macro and **host functions** calls used
///
/// This heading describes [`#[near]` on impl blocks](near#near-annotates-impl-blocks).
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// ```rust
/// # use near_sdk::near;
/// # #[near(contract_state)]
/// # pub struct Contract { /* .. */ }
/// #[near]
/// impl Contract {
///     pub fn view_method(&self) -> String { todo!("method body") }
///
///     pub fn mutating_method(&mut self, argument: String) { /* .. */ }
/// }
/// ```
///
/// ##### for above **view** method `#[near]` macro defines the following function:
///
/// ```rust,no_run
/// #[unsafe(no_mangle)]
/// pub extern "C" fn view_method() { /* .. */ }
/// ```
/// which
///
/// 1. calls [`env::setup_panic_hook`] host function
/// 2. calls [`env::state_read`] host function to load `Contract` into a `state` variable
///     1. `env::state_read`'s result is unwrapped with [`Option::unwrap_or_default`]
///     2. [`PanicOnDefault`] may be used to NOT let [implementation of `Default` for `Contract`](Default) value become the outcome `Contract`'s `state`, when [`env::state_read`] returns [`Option::None`]
/// 3. calls original `Contract::view_method(&state)` as defined in `#[near]` annotated [impl block](near#implementation-of-near-macro-and-host-functions-calls-used) and saves
///    the returned value into a `result` variable
/// 4. calls [`serde_json::to_vec`] on obtained `result` and saves returned value to `serialized_result` variable
///     1. `json` format can be changed to serializing with [`borsh::to_vec`] by using [`#[result_serializer(...)]`](`near#result_serializer-annotates-methods-of-a-type-in-its-impl-block`)
/// 5. if the `serialized_result` is an [`Result::Err`] error, then [`env::panic_str`] host function is called to signal result serialization error
/// 6. otherwise, if the `serialized_result` is a [`Result::Ok`], then [`env::value_return`] host function is called with unwrapped `serialized_result`
///
/// ##### for above **mutating** method `#[near]` macro defines the following function:
/// ```rust,no_run
/// #[unsafe(no_mangle)]
/// pub extern "C" fn mutating_method() { /* ..*/ }
/// ```
/// which
///
/// 1. calls [`env::setup_panic_hook`] host function
/// 2. calls [`env::input`] host function and saves it to `input` variable
/// 3. deserializes `Contract::mutating_method` arguments by calling [`serde_json::from_slice`] on `input` variable and saves it to `deserialized_input` variable
///     1. `json` format can be changed to deserializing with [`borsh::from_slice`] by using [`#[serializer(...)]`](`near#serializer-annotates-function-arguments`)
/// 4. if the `deserialized_input` is an [`Result::Err`] error, then [`env::panic_str`] host function is called to signal input deserialization error
/// 5. otherwise, if the `deserialized_input` is a [`Result::Ok`], `deserialized_input` is unwrapped and saved to `deserialized_input_success` variable
/// 6. calls [`env::state_read`] host function to load `Contract` into a `state` variable
///     1. `env::state_read`'s result is unwrapped with [`Option::unwrap_or_default`]
///     2. [`PanicOnDefault`] may be used to NOT let [implementation of `Default` for `Contract`](Default) value become the outcome `Contract`'s `state`, when [`env::state_read`] returns [`Option::None`]
/// 7. calls original `Contract::mutating_method(&mut state, deserialized_input_success.argument)` as defined in `#[near]` annotated [impl block](near#implementation-of-near-macro-and-host-functions-calls-used)
/// 8. calls [`env::state_write`] with `&state` as argument.
///
/// ---
///
/// ## Implementation of `#[callback_unwrap]` attribute and **host functions** calls used
///
/// This heading describes [`#[callback_unwrap]`](near#callback_unwrap-annotates-function-arguments).
///
/// In a nutshell and if the details of [ABI](https://github.com/near/abi) generation layer are put aside,
///
/// ```rust
/// # use near_sdk::near;
/// # #[near(contract_state)]
/// # pub struct Contract { /* .. */ }
/// #[near]
/// impl Contract {
///     pub fn method(
///         &mut self,
///         regular: String,
///         #[callback_unwrap] one: String,
///         #[callback_unwrap(max_bytes = 100)] two: String
///     ) { /* .. */ }
/// }
/// ```
///
/// For above `method` using the attribute on arguments, changes the body of function generated in  [`#[near]` on mutating method](near#for-above-mutating-method-near-macro-defines-the-following-function)
///
/// ```rust,no_run
/// #[unsafe(no_mangle)]
/// pub extern "C" fn method() { /* .. */ }
/// ```
///
/// in the following way:
///
/// 1. arguments, annotated with `#[callback_unwrap]`, are no longer expected to be included into `input`,
///    deserialized in (step **3**, [`#[near]` on mutating method](near#for-above-mutating-method-near-macro-defines-the-following-function)).
/// 2. for each argument, annotated with `#[callback_unwrap]`:
///     1. [`env::promise_result_checked`] host function is called with corresponding index, starting from 0
///        (`0u64` for argument `one`, `1u64` for argument `two` above), and saved into `promise_result` variable
///     2. if the `promise_result` is an `Err` (due to failed promise to too long result), then [`env::panic_str`]
///        host function is called to signal callback computation error
///     3. otherwise, if the `promise_result` is [`Ok`], it's unwrapped and saved to a `data` variable
///     4. `data` is deserialized similar to that as usual (step **3**, [`#[near]` on mutating method](near#for-above-mutating-method-near-macro-defines-the-following-function)),
///        and saved to `deserialized_n_promise` variable
/// 3. counterpart of (step **7**, [`#[near]` on mutating method](near#for-above-mutating-method-near-macro-defines-the-following-function)):
///    original method is called `Contract::method(&mut state, deserialized_input_success.regular, deserialized_0_promise, deserialized_1_promise)`,
///    as defined in `#[near]` annotated impl block
///
/// ---
pub use near_sdk_macros::near;

/// This macro is deprecated. Use [near] instead. The difference between `#[near]` and `#[near_bindgen]` is that
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

/// TODO: add docs
pub use near_sdk_macros::contract_error;

/// `ext_contract` takes a Rust Trait and converts it to a module with a struct and methods
/// for making cross-contract calls to an external contract. This enables asynchronous
/// communication between NEAR smart contracts.
///
/// # Module Name
///
/// - If no argument is provided (e.g., `#[ext_contract]`), the generated module name is the
///   trait name converted to snake_case (e.g., `ExtStatusMessage` becomes `ext_status_message`).
/// - If an argument is provided (e.g., `#[ext_contract(ext_calculator)]`), that name is used
///   as the module name.
///
/// # Generated Code
///
/// For a trait named `Calculator` with `#[ext_contract(ext_calculator)]`, the macro generates:
///
/// ## Module `ext_calculator`
///
/// A module containing the following:
///
/// ### Struct `CalculatorExt`
///
/// A builder struct for constructing cross-contract calls. This struct holds configuration
/// for the call (deposit, gas, etc.) and is marked with `#[must_use]` to ensure the call
/// is actually executed.
///
/// ### Functions
///
/// - **`ext(account_id: AccountId) -> CalculatorExt`** - Creates a new cross-contract call
///   builder targeting the specified `account_id`. This is the entry point for making calls
///   to an external contract.
///
/// - **`ext_on(promise: Promise) -> CalculatorExt`** - Creates a cross-contract call builder
///   that will be chained onto an existing [`Promise`]. Use this when you want to execute
///   the external call only after a previous promise completes (`.then()` semantics).
///
/// ### Builder Methods on `CalculatorExt`
///
/// These methods configure the cross-contract call and return `Self` for method chaining:
///
/// - **`with_attached_deposit(amount: NearToken) -> Self`** - Sets the amount of NEAR tokens
///   to attach to the call. Default is 0.
///
/// - **`with_static_gas(static_gas: Gas) -> Self`** - Sets the amount of gas to attach to
///   the call. This is the guaranteed minimum gas for the call. Default is 0.
///
/// - **`with_unused_gas_weight(gas_weight: u64) -> Self`** - Sets the weight for distributing
///   leftover gas from the current execution among scheduled calls. Higher weight means more
///   unused gas will be allocated to this call. Default uses [`GasWeight::default()`].
///
/// ### Trait Method Wrappers
///
/// For each method defined in the trait, a corresponding method is generated on `CalculatorExt`
/// that:
/// 1. Takes the same arguments as the trait method (excluding `&self`/`&mut self`).
/// 2. Serializes the arguments (JSON by default, or borsh if specified with `#[serializer(borsh)]`).
/// 3. Returns a [`Promise`] that represents the scheduled cross-contract call.
///
/// # Viewing Generated Method Signatures
///
/// To see the full signatures of generated methods in your contract's documentation, run
/// `cargo doc --open` in your contract crate. The generated module and its methods will
/// appear in your crate's documentation.
///
/// Note that the trait itself is preserved in the output, so the original trait methods
/// are also visible in the documentation.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use near_sdk::{AccountId, ext_contract, near, Promise, Gas};
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
///     pub fn multiply_by_five(&mut self, number: u64) -> Promise {
///         ext_calculator::ext(self.calculator_account.clone())
///             .with_static_gas(Gas::from_tgas(5))
///             .mult(number, 5)
///     }
/// }
/// ```
///
/// ## Default Module Name (Snake Case)
///
/// If no module name is provided, the trait name is converted to snake_case:
///
/// ```rust
/// use near_sdk::{AccountId, ext_contract, Promise};
///
/// // Module name will be `ext_status_message`
/// #[ext_contract]
/// pub trait ExtStatusMessage {
///     fn set_status(&mut self, message: String);
///     fn get_status(&self, account_id: near_sdk::AccountId) -> Option<String>;
/// }
///
/// fn example(account_id: AccountId) -> Promise {
///     ext_status_message::ext(account_id)
///         .set_status("Hello".to_string())
/// }
/// ```
///
/// ## Chaining Calls with `ext_on`
///
/// Use `ext_on` to chain a cross-contract call after an existing promise:
///
/// ```rust
/// use near_sdk::{ext_contract, near, AccountId, Promise};
///
/// #[ext_contract(ext_other)]
/// trait OtherContract {
///     fn step_one(&self);
///     fn step_two(&self);
/// }
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// struct Contract {}
///
/// #[near]
/// impl Contract {
///     pub fn chained_calls(&self, other_account: AccountId) -> Promise {
///         let first_call = ext_other::ext(other_account.clone()).step_one();
///         // step_two will only execute after step_one completes
///         ext_other::ext_on(first_call).step_two()
///     }
/// }
/// ```
///
/// ## Attaching Deposit
///
/// ```rust
/// use near_sdk::{ext_contract, AccountId, NearToken, Promise};
///
/// #[ext_contract(ext_ft)]
/// trait FungibleToken {
///     fn ft_transfer(&mut self, receiver_id: near_sdk::AccountId, amount: String, memo: Option<String>);
/// }
///
/// fn transfer_tokens(token_contract: AccountId, receiver: AccountId) -> Promise {
///     ext_ft::ext(token_contract)
///         .with_attached_deposit(NearToken::from_yoctonear(1)) // 1 yoctoNEAR for security
///         .ft_transfer(receiver, "1000000".to_string(), None)
/// }
/// ```
///
/// See more information about cross-contract calls in the [NEAR documentation](https://docs.near.org/build/smart-contracts/anatomy/crosscontract).
pub use near_sdk_macros::ext_contract;

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
/// Please use [near] instead.
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
/// It allows contract runtime to panic with the type using its [ToString] implementation
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
pub use promise::{Allowance, ConcurrentPromises, Promise, PromiseOrValue, ResumeError, YieldId};

// Private types just used within macro generation, not stable to be used.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

pub mod json_types;

mod types;
pub use crate::types::*;

pub mod events;
pub use crate::events::{AsNep297Event, Nep297Event};

pub mod state;

#[cfg(feature = "deterministic-account-ids")]
pub mod state_init;

pub mod errors;

#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub use environment::mock;
#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
// Re-export to avoid breakages
pub use environment::mock::MockedBlockchain;
#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub use environment::mock::test_vm_config;
#[cfg(all(feature = "unit-testing", not(target_arch = "wasm32")))]
pub use test_utils::context::VMContext;

pub mod utils;
pub use crate::utils::storage_key_impl::IntoStorageKey;
pub use crate::utils::*;

#[cfg(feature = "__macro-docs")]
pub mod near_annotations;

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
pub use serde_with;
