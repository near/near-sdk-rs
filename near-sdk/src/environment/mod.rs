pub mod env;

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
/// Mock blockchain utilities. These can only be used inside tests and are not available for
/// a wasm32 target.
pub mod mock;
