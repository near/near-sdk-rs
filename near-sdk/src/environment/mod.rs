pub mod env;

#[cfg(not(target_arch = "wasm32"))]
/// Mock blockchain utilities. These can only be used inside tests and are not available for
/// a wasm32 target.
pub mod mock;
