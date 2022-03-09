mod vm_types;
pub use self::vm_types::*;

mod public_key;
pub use self::public_key::{CurveType, PublicKey};

mod primitives;
pub use self::primitives::*;

mod account_id;
pub use self::account_id::{AccountId, ParseAccountIdError};

mod gas;
pub use self::gas::Gas;

/// Raw type for duration in nanoseconds
pub type Duration = u64;

/// Raw type for timestamp in nanoseconds
pub type Timestamp = u64;

/// Raw type for 32 bytes of the hash.
pub type CryptoHash = [u8; 32];

/// Balance of one Yocto NEAR, which is the smallest denomination. This value is 10^-24 of one NEAR.
pub const ONE_YOCTO: Balance = 1;

/// Balance of one NEAR, which is 10^24 Yocto NEAR.
pub const ONE_NEAR: Balance = 1_000_000_000_000_000_000_000_000;
