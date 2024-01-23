use near_sdk::serde::{Deserialize, Serialize};

/// The contract source metadata is a standard interface that allows auditing and viewing source code for a deployed smart contract.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Standard {
    pub standard: String,
    pub version: String,
}
