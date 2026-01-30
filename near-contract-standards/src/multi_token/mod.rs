//! Multi tokens as described in [NEP-245](https://github.com/near/NEPs/blob/master/neps/nep-0245.md).
//!
//! This module represents a Multi Token standard (NEP-245) which supports fungible,
//! semi-fungible, non-fungible, and tokens of any type, allowing for ownership,
//! transfer, and batch transfer of tokens regardless of specific type.
//!
//! # Example Usage
//!
//! ```ignore
//! use near_contract_standards::multi_token::{MultiToken, Token, TokenId};
//! use near_contract_standards::multi_token::core::MultiTokenCore;
//! use near_contract_standards::multi_token::metadata::{MTContractMetadata, MTTokenMetadata};
//! use near_sdk::{near, AccountId, PanicOnDefault, BorshStorageKey};
//! use near_sdk::collections::LazyOption;
//!
//! #[derive(BorshStorageKey)]
//! #[near]
//! enum StorageKey {
//!     MultiToken,
//!     Metadata,
//!     TokenMetadata,
//!     Enumeration,
//!     Approval,
//! }
//!
//! #[derive(PanicOnDefault)]
//! #[near(contract_state)]
//! pub struct Contract {
//!     tokens: MultiToken,
//!     metadata: LazyOption<MTContractMetadata>,
//! }
//!
//! #[near]
//! impl Contract {
//!     #[init]
//!     pub fn new(owner_id: AccountId) -> Self {
//!         Self {
//!             tokens: MultiToken::new(
//!                 StorageKey::MultiToken,
//!                 owner_id,
//!                 Some(StorageKey::TokenMetadata),
//!                 Some(StorageKey::Enumeration),
//!                 Some(StorageKey::Approval),
//!             ),
//!             metadata: LazyOption::new(StorageKey::Metadata, None),
//!         }
//!     }
//! }
//! ```

/// The [core multi token standard](https://nomicon.io/Standards/Tokens/MultiToken/Core).
/// This is the base standard with transfer methods.
pub mod core;

/// Approval management for multi tokens.
pub mod approval;

/// Trait for the [MT enumeration standard](https://nomicon.io/Standards/Tokens/MultiToken/Enumeration).
/// This provides useful view-only methods returning token supply, tokens by owner, etc.
pub mod enumeration;

/// Standard events for multi token operations.
pub mod events;

/// Metadata traits and implementation according to the [MT metadata standard](https://nomicon.io/Standards/Tokens/MultiToken/Metadata).
/// This covers both the contract metadata and the individual token metadata.
pub mod metadata;

/// The Token struct and related types for the multi token.
mod token;

/// Utility functions for multi token operations.
mod utils;

// Re-export main types at crate level for convenience
pub use self::token::{Approval, ClearedApproval, Token, TokenId};

// Re-export core types
pub use self::core::MultiToken;
pub use self::core::MultiTokenCore;
pub use self::core::MultiTokenReceiver;
pub use self::core::MultiTokenResolver;

// Re-export enumeration types
pub use self::enumeration::MultiTokenEnumeration;
pub use self::enumeration::MultiTokenEnumerationMetadata;

// Re-export approval types
pub use self::approval::MultiTokenApproval;

// Re-export utilities
pub use self::utils::*;
