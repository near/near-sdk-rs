mod vm_types;
pub use self::vm_types::*;

mod public_key;
pub use self::public_key::{CurveType, PublicKey};

mod primitives;
pub use self::primitives::*;

mod account_id;
pub use self::account_id::{AccountId, ParseAccountIdError};

/// Raw type for duration in nanoseconds
pub type Duration = u64;

/// Raw type for timestamp in nanoseconds
pub type Timestamp = u64;

/// Raw type for 32 bytes of the hash.
pub type CryptoHash = [u8; 32];
