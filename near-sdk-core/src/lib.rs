pub mod json_types;

pub mod types;

pub mod allowance;

pub use base64;
pub use bs58;

#[cfg(any(near, feature = "serde"))]
pub use serde;

#[cfg(any(near, feature = "serde"))]
pub use serde_json;

#[cfg(any(near, feature = "serde"))]
pub use serde_with;

#[cfg(any(near, feature = "borsh"))]
pub use borsh;

#[cfg(feature = "abi")]
pub use schemars;

#[cfg(test)]
// XXX: `near-sdk` was added in order to enable tests and doctests compiling with mockchain
use near_sdk as _;
