use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}
