#[cfg(all(feature = "near-contracts", feature = "digest"))]
compile_error!(
    "features `near-contracts` and `digest` are mutually exclusive - both provide keccak256 for `derive-account-id`"
);

mod global_contract_identifier;
pub use global_contract_identifier::*;

mod state_init;
pub use state_init::*;
