use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "standard")]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

impl Default for ContractSourceMetadata {
    // TODO: Use Cargo.toml from CARGO_MANIFEST_DIR to populate the version field.
    fn default() -> Self {
        Self { version: None, link: None, standards: vec![] }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}
