//! `#[near]` and `#[near_bindgen]` documentation module
//!
//! This is not a real module; here we document the attributes that [`#[near]`](../attr.near.html)
//! and [`#[near_bindgen]`](../attr.near_bindgen.html) macro use.

/// Initialization Methods inner [`#[near]`](../attr.near.html) annotation. More details can be found [here](https://docs.near.org/sdk/rust/contract-structure/near-bindgen#initialization-methods)
///
/// By default, the `Default::default()` implementation of a contract will be used to initialize a contract. There can be a custom initialization function which takes parameters or performs custom logic with the following `#[init]` annotation:
/// # Examples
///
/// ## Basic example
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
pub fn init() {}

/// Payable Methods inner [`#[near]`](../attr.near.html) annotation. More details can be found [here](https://docs.near.org/sdk/rust/contract-structure/near-bindgen#payable-methods)
///
/// Methods can be annotated with `#[payable]` to allow tokens to be transferred with the method invocation. For more information, see payable methods.
///
/// To declare a function as payable, use the `#[payable]` annotation as follows:
/// # Examples
///
/// ## Basic example
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
pub fn payable() {}

/// Private Methods inner [`#[near]`](../attr.near.html) annotation. More details can be found [here](https://docs.near.org/sdk/rust/contract-structure/near-bindgen#private-methods)
///
/// Some methods need to be exposed to allow the contract to call a method on itself through a promise, but want to disallow any other contract to call it. For this, use the `#[private]` annotation to panic when this method is called externally. See [private methods](https://docs.near.org/sdk/rust/contract-interface/private-methods) for more information.
///
/// This annotation can be applied to any method through the following:
/// # Examples
///
/// ## Basic example
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
pub fn private() {}

/// Result serialization inner [`#[near]`](../attr.near.html) annotation.
///
/// Only one of `borsh` or `json` can be specified.
///
/// # Examples
///
/// ## Basic example
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
///     #[result_serializer(borsh)]
///     pub fn add_borsh(&self, #[serializer(borsh)] _a: Vec<String>) {
///         // ..
///     }
/// }
/// ```
pub fn result_serializer() {}

/// Support Result types regardless of how they're referred to inner [`#[near]`](../attr.near.html) annotation.
///
/// Have `#[handle_result]` to Support Result types regardless of how they're referred to
/// Function marked with `#[handle_result]` should return `Result<T, E> (where E implements FunctionError)`. If you're trying to use a type alias for `Result`, try `#[handle_result(aliased)]`
///
/// # Examples
///
/// ## Basic example
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
///     pub fn get_result(
///         &self,
///         account_id: AccountId,
///         #[callback_result] set_status_result: Result<(), PromiseError>,
///     ) -> Result<(), &'static str> {
///         // ..
///         Ok(())
///     }
/// }
/// ```
pub fn handle_result() {}
