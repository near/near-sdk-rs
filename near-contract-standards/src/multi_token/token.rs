use crate::multi_token::metadata::TokenMetadata;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::serde::{Deserialize, Serialize};
pub use near_sdk::{AccountId, Balance};
use std::collections::HashMap;

/// Type alias for convenience
pub type TokenId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Approval {
    pub amount: u128,
    pub approval_id: u64,
}

// How Approvals are stored in the contract
pub type ApprovalContainer = LookupMap<TokenId, HashMap<AccountId, HashMap<AccountId, Approval>>>;

// Represents a temporary record of an Approval
// that was removed from the ApprovalContainer but may be restored in case of rollback in XCC.
// Values are (owner_id, approval_id, amount)
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
