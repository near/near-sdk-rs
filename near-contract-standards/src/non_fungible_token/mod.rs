/// Trait for the [approval management standard](https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html) for NFTs.
pub mod approval;

/// Common implementation of the [approval management standard](https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html) for NFTs.
/// Approval receiver is the trait for the method called (or attempted to be called)
/// on the contract/account that has just been approved. This is not required to implement.

/// Trait for the [core non-fungible token standard](https://nomicon.io/Standards/NonFungibleToken/Core.html).
pub mod core;
/// Common implementation of the [core non-fungible token standard](https://nomicon.io/Standards/NonFungibleToken/Core.html).
/// Trait for the [NFT enumeration standard](https://nomicon.io/Standards/NonFungibleToken/Enumeration.html).
/// This provides useful view-only methods returning token supply, tokens by owner, etc.
pub mod enumeration;
/// Macros typically used by a contract wanting to take advantage of the non-fungible
/// token NEAR contract standard approach.
mod macros;
/// Metadata traits and implementation according to the [NFT enumeration standard](https://nomicon.io/Standards/NonFungibleToken/Metadata.html).
/// This covers both the contract metadata and the individual token metadata.
pub mod metadata;
/// Trait for the receiver of an `nft_transfer_call` method call. This method is
/// implemented on a receiving account separate from the NFT contract. This is
/// part of the core standard.
/// Trait for the resolver of an `nft_transfer_call` method call. This method is
/// implemented on the NFT contract and used to resolve how the transfer-and-call
/// process went. This is part of the core standard.

/// The Token struct for the non-fungible token.
mod token;
pub use self::token::{Token, TokenId};

/// NFT utility functions
mod utils;
pub use utils::*;

pub use self::core::NonFungibleToken;
pub use macros::*;
