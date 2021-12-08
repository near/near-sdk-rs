use crate::test_utils::VMContextBuilder;
use crate::{env, mock::MockedBlockchain, AccountId, VMConfig};
use near_primitives_core::runtime::fees::RuntimeFeesConfig;

pub fn alice() -> AccountId {
    AccountId::new_unchecked("alice.near".to_string())
}

pub fn bob() -> AccountId {
    AccountId::new_unchecked("bob.near".to_string())
}

pub fn carol() -> AccountId {
    AccountId::new_unchecked("carol.near".to_string())
}

/// Updates the blockchain interface with the config passed in.
// TODO(austinabell): This seems like a footgun, not clear it's replacing the context with default
pub fn setup_with_config(vm_config: VMConfig) {
    let context = VMContextBuilder::new().build();
    let storage = crate::mock::with_mocked_blockchain(|b| b.take_storage());
    env::set_blockchain_interface(MockedBlockchain::new(
        context,
        vm_config,
        RuntimeFeesConfig::test(),
        vec![],
        storage,
        Default::default(),
        None,
    ));
}

/// Setup the blockchain interface with a default configuration.
pub fn setup() {
    setup_with_config(VMConfig::test());
}

// free == effectively unlimited gas
/// Sets up the blockchain interface with a [`VMConfig`] which sets the gas costs to zero.
pub fn setup_free() {
    setup_with_config(VMConfig::free());
}
