mod vm_types;
pub use self::vm_types::*;

mod public_key;
pub use self::public_key::{CurveType, PublicKey};

mod primitives;
pub use self::primitives::*;

#[cfg(feature = "global-contracts")]
mod contract_code;
#[cfg(feature = "global-contracts")]
pub use contract_code::*;

pub use near_account_id::{self as account_id, AccountId, AccountIdRef};
/// A wrapper struct for `u64` that represents gas. And provides helpful methods to convert to and from tera-gas and giga-gas.
pub use near_gas::NearGas as Gas;
/// A wrapper struct for `u128` that represents tokens. And provides helpful methods to convert with a proper precision.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GasWeight(pub u64);

impl Default for GasWeight {
    fn default() -> Self {
        Self(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_weight_clone() {
        let weight = GasWeight(42);
        // Deliberately test Clone on a Copy type
        #[allow(clippy::clone_on_copy)]
        let cloned = weight.clone();
        assert_eq!(weight, cloned);
    }

    #[test]
    fn test_gas_weight_copy() {
        let weight = GasWeight(42);
        let copied = weight; // Copy
        assert_eq!(weight.0, copied.0);
        // `weight` is still usable after copy
        assert_eq!(weight.0, 42);
    }

    #[test]
    fn test_gas_weight_default() {
        let weight = GasWeight::default();
        assert_eq!(weight.0, 1);
    }

    #[test]
    fn test_gas_weight_debug() {
        let weight = GasWeight(100);
        assert_eq!(format!("{:?}", weight), "GasWeight(100)");
    }

    #[test]
    fn test_gas_weight_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(GasWeight(1));
        set.insert(GasWeight(2));
        set.insert(GasWeight(1)); // duplicate
        assert_eq!(set.len(), 2);
    }
}
