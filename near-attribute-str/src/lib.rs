/// Initialization Methods.
///
/// By default, the Default::default() implementation of a contract will be used to initialize a contract. There can be a custom initialization function which takes parameters or performs custom logic with the following #[init] annotation:
/// # Examples
///
/// ## Basic example
///
/// ```rust
///#[near]
///impl Counter {
///    #[init]
///    pub fn new(value: u64) -> Self {
///        log!("Custom counter initialization!");
///        Self { value }
///    }
///}
/// ```
#[allow(non_upper_case_globals)]
pub const init: &str = "init";

/// Payable Methods
///
/// Methods can be annotated with #[payable] to allow tokens to be transferred with the method invocation. For more information, see payable methods.
///
/// To declare a function as payable, use the #[payable] annotation as follows:
/// # Examples
///
/// ## Basic example
///
/// ```rust
///#[payable]
///pub fn my_method(&mut self) {
///    ...
///}
/// ```
#[allow(non_upper_case_globals)]
pub const payable: &str = "payable";

/// Private Methods
///
/// Some methods need to be exposed to allow the contract to call a method on itself through a promise, but want to disallow any other contract to call it. For this, use the #[private] annotation to panic when this method is called externally. See [private methods](https://docs.near.org/sdk/rust/contract-interface/private-methods) for more information.
///
/// This annotation can be applied to any method through the following:
/// # Examples
///
/// ## Basic example
///
/// ```rust
/// #[private]
/// pub fn my_method(&mut self) {
///    ...
///}
/// ```
#[allow(non_upper_case_globals)]
pub const private: &str = "private";

/// Result serialization.
///
/// Only one of `borsh` or `json` can be specified.
///
/// # Examples
///
/// ## Basic example
///
/// ```rust
///#[result_serializer(borsh)]
///pub fn add_borsh(&self, #[serializer(borsh)] a: Pair, #[serializer(borsh)] b: Pair) -> Pair {
///    sum_pair(&a, &b)
///}
/// ```
#[allow(non_upper_case_globals)]
pub const result_serializer: &str = "result_serializer";

/// Handle result
#[allow(non_upper_case_globals)]
pub const handle_result: &str = "handle_result";
