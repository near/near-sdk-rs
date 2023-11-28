mod vm_types;
pub use self::vm_types::*;

mod public_key;
pub use self::public_key::{CurveType, PublicKey};

mod primitives;
pub use self::primitives::*;

pub use near_account_id::{AccountId, AccountIdRef};
pub use near_gas::NearGas as Gas;
pub use near_token::NearToken;

mod error;
pub use self::error::Abort;
pub use self::error::FunctionError;

/// Raw type for duration in nanoseconds
pub type Duration = u64;

/// Raw type for timestamp in nanoseconds
pub type Timestamp = u64;

/// Raw type for 32 bytes of the hash.
pub type CryptoHash = [u8; 32];

/// Weight of unused gas to use with [`promise_batch_action_function_call_weight`].
///
/// This weight will be used relative to other weights supplied in the function to distribute
/// unused gas to those function calls. The default weight is 1.
///
/// For example, if 40 gas is leftover from the current method call and three functions specify
/// the weights 1, 5, 2 then 5, 25, 10 gas will be added to each function call respectively,
/// using up all remaining available gas.
///
/// [`promise_batch_action_function_call_weight`]: `crate::env::promise_batch_action_function_call_weight`
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct GasWeight(pub u64);

impl Default for GasWeight {
    fn default() -> Self {
        Self(1)
    }
}
