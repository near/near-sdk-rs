pub use near_bindgen_macros::near_bindgen;

pub mod collections;
mod environment;
pub use environment::env;

#[cfg(feature = "testing")]
pub use environment::mocked_blockchain::MockedBlockchain;
#[cfg(feature = "testing")]
pub use near_vm_logic::types::PromiseResult;
#[cfg(feature = "testing")]
pub use near_vm_logic::Config;
#[cfg(feature = "testing")]
pub use near_vm_logic::VMContext;

#[cfg(feature = "testing")]
#[macro_export]
macro_rules! testing_env {
    ($context:ident, $config:expr) => {
            near_bindgen::env::set_blockchain_interface(Box::new(MockedBlockchain::new($context, $config, vec![]
        )));
    };
}

pub use environment::blockchain_interface::BlockchainInterface;
