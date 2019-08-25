pub use near_bindgen_macros::near_bindgen;

pub mod collections;
mod environment;
pub use environment::blockchain_interface::BlockchainInterface;
pub use environment::environment::Environment;
pub use environment::mocked_blockchain::MockedBlockchain;
