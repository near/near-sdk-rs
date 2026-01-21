use near_sdk::{
    serde::{Deserialize, Serialize},
    AccountId, NearSchema,
};

/// Note that token IDs for multi tokens are strings on NEAR. It's still fine to use autoincrementing numbers as unique IDs if desired, but they should be stringified.
pub type TokenId = String;

/// The Token struct for the multi token.
#[derive(NearSchema, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    pub token_id: TokenId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<AccountId>,
}
