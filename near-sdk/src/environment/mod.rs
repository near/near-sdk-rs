#[cfg(not(target_arch = "wasm32"))]
pub mod blockchain_interface;
pub mod env;
mod sys;

#[cfg(not(target_arch = "wasm32"))]
pub mod mocked_blockchain;
