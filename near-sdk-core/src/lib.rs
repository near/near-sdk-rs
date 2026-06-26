#[cfg(any(doctest, all(test, feature = "__near-sdk-unit-testing")))]
// XXX: `near-sdk` was added in order to enable tests and doctests compiling with mockchain
use near_sdk as _;

macro_rules! abort {
    () => {{
        #[cfg(any(near, feature = "__near-sdk-unit-testing"))]
        {
            near_sdk_env::abort()
        }
        #[cfg(not(any(near, feature = "__near-sdk-unit-testing")))]
        {
            panic!()
        }
    }};
}

pub mod json_types;

pub mod types;

pub mod allowance;

pub mod events;

pub use bs58;

#[cfg(feature = "serde")]
pub use base64;

#[cfg(feature = "serde")]
pub use serde;

#[cfg(feature = "serde")]
pub use serde_json;

#[cfg(feature = "serde")]
pub use serde_with;

#[cfg(feature = "borsh")]
pub use borsh;
