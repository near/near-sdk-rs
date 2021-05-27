/// Trait for the [approval management standard](https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html) for NFTs.
pub mod approval;
/// Common implementation of the [approval management standard](https://nomicon.io/Standards/NonFungibleToken/ApprovalManagement.html) for NFTs.
pub mod approval_impl;
/// Approval receiver is the trait for the method called (or attempted to be called)
/// on the contract/account that has just been approved. This is not required to implement.
pub mod approval_receiver;
/// Trait for the [core non-fungible token standard](https://nomicon.io/Standards/NonFungibleToken/Core.html).
pub mod core;
/// Common implementation of the [core non-fungible token standard](https://nomicon.io/Standards/NonFungibleToken/Core.html).
pub mod core_impl;
/// Trait for the [NFT enumeration standard](https://nomicon.io/Standards/NonFungibleToken/Enumeration.html).
/// This provides useful view-only methods returning token supply, tokens by owner, etc.
pub mod enumeration;
/// Common implementation of the [NFT enumeration standard](https://nomicon.io/Standards/NonFungibleToken/Enumeration.html).
pub mod enumeration_impl;
/// Macros typically used by a contract wanting to take advantage of the non-fungible
/// token NEAR contract standard approach.
pub mod macros;
/// Metadata traits and implementation according to the [NFT enumeration standard](https://nomicon.io/Standards/NonFungibleToken/Metadata.html).
/// This covers both the contract metadata and the individual token metadata.
pub mod metadata;
/// Trait for the receiver of an `nft_transfer_call` method call. This method is
/// implemented on a receiving account separate from the NFT contract. This is
/// part of the core standard.
pub mod receiver;
/// Trait for the resolver of an `nft_transfer_call` method call. This method is
/// implemented on the NFT contract and used to resolve how the transfer-and-call
/// process went. This is part of the core standard.
pub mod resolver;
/// The Token struct for the non-fungible token.
pub mod token;
/// NFT utility functions
pub mod utils;

pub use core_impl::NonFungibleToken;
pub use macros::*;
