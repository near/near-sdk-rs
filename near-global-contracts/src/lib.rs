mod global_contract_identifier;
pub use global_contract_identifier::*;

mod state_init;
pub use state_init::*;

#[cfg(test)]
// XXX: `near-sdk` was added in order to enable tests and doctests compiling with mockchain
use near_sdk as _;
