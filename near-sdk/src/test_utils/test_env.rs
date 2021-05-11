use crate::test_utils::VMContextBuilder;
use crate::{env, MockedBlockchain};
use near_vm_logic::types::AccountId;
use near_vm_logic::VMConfig;

pub fn alice() -> AccountId {
    "alice.near".to_string()
}

pub fn bob() -> AccountId {
    "bob.near".to_string()
}

pub fn carol() -> AccountId {
    "carol.near".to_string()
}

/// Updates the blockchain interface with the config passed in.
pub fn setup_with_config(vm_config: VMConfig) {
    let context = VMContextBuilder::new().build();
    let storage = match env::take_blockchain_interface() {
        Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
        None => Default::default(),
    };
    env::set_blockchain_interface(Box::new(MockedBlockchain::new(
        context,
        vm_config,
        Default::default(),
        vec![],
        storage,
        Default::default(),
        None,
    )));
}

/// Setup the blockchain interface with a default configuration.
pub fn setup() {
    setup_with_config(VMConfig::default());
}

// free == effectively unlimited gas
/// Sets up the blockchain interface with a [`VMConfig`] which sets the gas costs to zero.
pub fn setup_free() {
    setup_with_config(VMConfig::free());
}
