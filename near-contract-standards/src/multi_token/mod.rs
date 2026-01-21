//! Multi tokens as described in [NEP-245](https://github.com/near/NEPs/blob/master/neps/nep-0245.md).
//!
//! This module represents a Multi Token standard (NEP-245) which supports fungible,
//! semi-fungible, non-fungible, and tokens of any type, allowing for ownership,
//! transfer, and batch transfer of tokens regardless of specific type.

/// The [core multi token standard](https://nomicon.io/Standards/Tokens/MultiToken/Core).
/// This is the base standard with transfer methods.
pub mod core;

/// Trait for the [MT enumeration standard](https://nomicon.io/Standards/Tokens/MultiToken/Enumeration).
/// This provides useful view-only methods returning token supply, tokens by owner, etc.
pub mod enumeration;

pub mod events;

/// Metadata traits and implementation according to the [MT metadata standard](https://nomicon.io/Standards/Tokens/MultiToken/Metadata).
/// This covers both the contract metadata and the individual token metadata.
pub mod metadata;

/// The Token struct for the multi token.
mod token;
pub use self::token::{Token, TokenId};

pub use self::core::resolver::ClearedApproval;
pub use self::core::MultiTokenCore;
pub use self::core::MultiTokenReceiver;
pub use self::core::MultiTokenResolver;
pub use self::enumeration::MultiTokenEnumeration;
pub use self::enumeration::MultiTokenEnumerationMetadata;
