use crate::test_utils::VMContextBuilder;
use crate::{test_vm_config, testing_env, AccountId};

pub fn alice() -> AccountId {
    "alice.near".parse().unwrap()
}

pub fn bob() -> AccountId {
    "bob.near".parse().unwrap()
}

pub fn carol() -> AccountId {
    "carol.near".parse().unwrap()
}

/// Updates the blockchain interface with the config passed in.
#[deprecated(
    since = "4.0.0",
    note = "Use `testing_env!` macro to initialize with specific VMConfig"
)]
pub fn setup_with_config(vm_config: near_parameters::vm::Config) {
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
    let mut config = test_vm_config();
    config.make_free();
    crate::testing_env!(VMContextBuilder::new().build(), config)
}
