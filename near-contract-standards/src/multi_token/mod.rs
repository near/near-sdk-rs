//! Multi tokens as described in [NEP-245](https://github.com/near/NEPs/blob/master/neps/nep-0245.md).
//!
//! This module represents a Multi Token standard (NEP-245) which supports fungible,
//! semi-fungible, non-fungible, and tokens of any type, allowing for ownership,
//! transfer, and batch transfer of tokens regardless of specific type.
//!
//! # Example Usage
//!
//! ```ignore
//! use near_contract_standards::multi_token::{
//!     MultiToken, MultiTokenCore, Token, TokenId,
//!     MTContractMetadata, MT_METADATA_SPEC,
//! };
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
pub use self::approval::MultiTokenApprovalReceiver;

// Re-export metadata types for convenience
pub use self::metadata::MTBaseTokenMetadata;
pub use self::metadata::MTContractMetadata;
pub use self::metadata::MTTokenMetadata;
pub use self::metadata::MTTokenMetadataAll;
pub use self::metadata::MultiTokenMetadataProvider;
pub use self::metadata::MT_METADATA_SPEC;

// Re-export utilities
pub use self::utils::*;

// Re-export event types
pub use self::events::MtBurn;
pub use self::events::MtMint;
pub use self::events::MtTransfer;
pub use self::events::REFUND_MEMO;
pub use self::events::REFUND_MEMO_EXTRA_BYTES;
pub use self::events::TOTAL_LOG_LENGTH_LIMIT;
