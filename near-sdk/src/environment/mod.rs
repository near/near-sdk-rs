pub mod blockchain_interface;
pub mod env;
#[cfg(not(target_arch = "wasm32"))]
pub mod mocked_blockchain;
