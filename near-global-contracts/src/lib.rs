#[cfg(all(test, feature = "__near-sdk-unit-testing"))]
// XXX: `near-sdk` was added in order to enable doctests and tests that require mockchain
use near_sdk as _;

mod global_contract_identifier;
pub use global_contract_identifier::*;

mod state_init;
pub use state_init::{StateInit, StateInitV1};
