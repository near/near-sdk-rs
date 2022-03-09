use crate::test_utils::VMContextBuilder;
use crate::{testing_env, AccountId, VMConfig};

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
#[deprecated(
    since = "4.0.0",
    note = "Use `testing_env!` macro to initialize with specific VMConfig"
)]
pub fn setup_with_config(vm_config: VMConfig) {
    testing_env!(VMContextBuilder::new().build(), vm_config)
}

/// Setup the blockchain interface with a default configuration.
#[deprecated(
    since = "4.0.0",
    note = "Mocked blockchain is now setup by default, use `testing_env!`"
)]
pub fn setup() {
    testing_env!(VMContextBuilder::new().build());
}

/// free == effectively unlimited gas
/// Sets up the blockchain interface with a [`VMConfig`] which sets the gas costs to zero.
pub fn setup_free() {
    crate::testing_env!(VMContextBuilder::new().build(), VMConfig::free())
}
