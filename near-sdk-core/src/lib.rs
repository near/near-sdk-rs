pub mod json_types;

pub mod types;

pub mod allowance;

pub use base64;
pub use borsh;
pub use bs58;
#[cfg(feature = "abi")]
pub use schemars;
pub use serde;
pub use serde_json;
pub use serde_with;

#[cfg(test)]
// XXX: `near-sdk` was added in order to enable tests and doctests compiling with mockchain
use near_sdk as _;
