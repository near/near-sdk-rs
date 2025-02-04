//! `#[near]` and `#[near_bindgen]` documentation module
//!
//! This is not a real module; here we document the attributes that [`#[near]`](crate::near)
//! and [`#[near_bindgen]`](crate::near_bindgen) macro use.

/// See [`near_sdk::near #[init]`](macro@crate::near#init-sub-attribute)
pub fn init() {}

/// See [`near_sdk::near #[payable]`](macro@crate::near#payable-sub-attribute)
pub fn payable() {}

/// See [`near_sdk::near #[private]`](macro@crate::near#private-sub-attribute)
pub fn private() {}

/// See [`near_sdk::near #[result_serializer]`](macro@crate::near#result_serializer-sub-attribute)
pub fn result_serializer() {}

/// See [`near_sdk::near #[handle_result]`](macro@crate::near#handle_result-sub-attribute)
pub fn handle_result() {}

/// See [`near_sdk::near #[event_json]`](macro@crate::near#event_json-sub-attribute)
pub fn event_json() {}

/// See [`near_sdk::near #[contract_metadata]`](macro@crate::near#contract_metadata-sub-attribute)
pub fn contract_metadata() {}
