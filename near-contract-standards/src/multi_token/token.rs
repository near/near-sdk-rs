use crate::multi_token::metadata::MTTokenMetadata;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, NearSchema,
};
use std::collections::HashMap;

/// Note that token IDs for multi tokens are strings on NEAR. It's still fine
/// to use autoincrementing numbers as unique IDs if desired, but they should
/// be stringified. This is to make IDs more future-proof as chain-agnostic
/// conventions and standards arise.
pub type TokenId = String;

/// Approval information for a specific grant.
/// Stores the approval ID and the amount approved for transfer.
#[derive(
    NearSchema,
    Debug,
    Clone,
    PartialEq,
    Eq,
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Approval {
    /// Unique identifier for this approval, used to prevent race conditions
    pub approval_id: u64,
    /// The amount of tokens approved for transfer
    pub amount: u128,
}

/// Cleared approval info stored during cross-contract calls for potential rollback.
///
/// This is a tuple of `(account_id, approval_id, amount)` where:
/// - `account_id`: The account that was approved to transfer tokens
/// - `approval_id`: The unique approval ID for this grant
/// - `amount`: The amount that was approved for transfer
///
/// This format matches the NEP-245 spec for the `approvals` parameter in `mt_resolve_transfer`.
pub type ClearedApproval = (AccountId, u64, u128);

/// The Token struct returned by view methods.
///
/// In this implementation, the Token struct takes metadata and approval extensions
/// as optional fields, as they are frequently used in modern multi-token contracts.
#[derive(NearSchema, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    /// The unique identifier for this token
    pub token_id: TokenId,

    /// The owner of this token. For fungible-style tokens where multiple accounts
    /// hold balances, this may be `None`. For NFT-style tokens (supply=1), this
    /// will be `Some(owner)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<AccountId>,

    /// Token-specific metadata, if the metadata extension is used
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MTTokenMetadata>,

    /// Approved accounts for this token, if the approval extension is used.
    /// Maps account_id to their Approval info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_account_ids: Option<HashMap<AccountId, Approval>>,
}

impl Token {
    /// Create a new Token with just the ID
    pub fn new(token_id: TokenId) -> Self {
        Self { token_id, owner_id: None, metadata: None, approved_account_ids: None }
    }

    /// Set the owner
    pub fn with_owner(mut self, owner_id: AccountId) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    /// Set the metadata
    pub fn with_metadata(mut self, metadata: MTTokenMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set the approved accounts
    pub fn with_approved_account_ids(
        mut self,
        approved_account_ids: HashMap<AccountId, Approval>,
    ) -> Self {
        self.approved_account_ids = Some(approved_account_ids);
        self
    }
}
