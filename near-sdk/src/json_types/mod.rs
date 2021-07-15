//! Helper types for JSON serialization.

mod hash;
mod integers;
mod vector;

use crate::types::{AccountId, PublicKey};

pub use hash::Base58CryptoHash;
pub use integers::{I128, I64, U128, U64};
pub use vector::Base64VecU8;

#[deprecated(
    since = "4.0.0",
    note = "ValidAccountId is no longer maintained, and AccountId should be used instead"
)]
pub type ValidAccountId = AccountId;

// This deprecated attribute doesn't work for the current rust version (1.52)
// but will likely work in the future. Also included just to note that it is
// indeed deprecated and not just a random export.
#[deprecated(
    since = "4.0.0",
    note = "This import is deprecated. Best to import directly from near_sdk"
)]
pub use crate::types::CurveType;

#[deprecated(
    since = "4.0.0",
    note = "PublicKey type is now unified with Base58PublicKey. It is \
            recommended to use PublicKey going forward to avoid using \
            similar sounding types for the same thing."
)]
pub type Base58PublicKey = PublicKey;
