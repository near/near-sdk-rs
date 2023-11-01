/// The contract source metadata is a standard interface that allows auditing and viewing source code for a deployed smart contract.
#[derive(Debug, serde:: Deserialize, PartialEq)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}
