/// The [approval management standard](https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html) for NFTs.
pub mod approval;
/// The [core non-fungible token standard](https://nomicon.io/Standards/NonFungibleToken/Core.html). This can be though of as the base standard, with the others being extension standards.
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
/// The Token struct for the non-fungible token.
mod token;
pub use self::token::{Token, TokenId};

/// NFT utility functions
mod utils;
pub use utils::*;

pub use self::approval::NonFungibleTokenApproval;
pub use self::core::NonFungibleToken;
pub use self::core::NonFungibleTokenResolver;
pub use self::enumeration::NonFungibleTokenEnumeration;

pub mod events;
