use crate::multi_token::metadata::TokenMetadata;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
pub use near_sdk::{AccountId, Balance};
use std::collections::HashMap;

/// Type alias for convenience
pub type TokenId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Approval {
    pub amount: u128,
    pub approval_id: u64,
}

// How Approvals are stored in the contract
pub type ApprovalContainer = LookupMap<TokenId, HashMap<AccountId, HashMap<AccountId, Approval>>>;

// Represents a record of an Approval that has been temporarily added or removed
// from the ApprovalContainer during cross-contract calls (XCC).
// This data is stored to facilitate possible rollback scenarios where the
// approval needs to be restored.
//
// The tuple contains the following elements:
// - `AccountId`: The Account ID of the owner who initially granted the approval.
// - `Approval`: A struct containing:
//   - `amount`: The number of tokens that were initially approved for transfer.
//   - `approval_id`: A unique identifier assigned to this specific approval.
pub type ClearedApproval = (AccountId, Approval);

/// Info on individual token
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "abi", derive(schemars::JsonSchema))]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    pub token_id: String,
    pub owner_id: AccountId,
    /// Total amount generated
    pub supply: u128,
    pub metadata: Option<TokenMetadata>,
}
