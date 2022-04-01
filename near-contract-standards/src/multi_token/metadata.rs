use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::require;
use near_sdk::serde::{Deserialize, Serialize};

/// Version of standard
pub const MT_METADATA_SPEC: &str = "mt-0.0.1";

/// Metadata that will be permanently set at the contract init
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct MtContractMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub base_uri: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
}

/// Metadata for each token
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetadata {
    pub title: Option<String>,
    /// Free-form description
    pub description: Option<String>,
    /// URL to associated media, preferably to decentralized, content-addressed storage
    pub media: Option<String>,
    /// Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
    pub media_hash: Option<String>,
    /// When token was issued or minted, Unix epoch in milliseconds
    pub issued_at: Option<String>,
    /// When token expires, Unix epoch in milliseconds
    pub expires_at: Option<String>,
    /// When token starts being valid, Unix epoch in milliseconds
    pub starts_at: Option<String>,
    /// When token was last updated, Unix epoch in milliseconds
    pub updated_at: Option<String>,
    /// Anything extra the MT wants to store on-chain. Can be stringified JSON.
    pub extra: Option<String>,
    /// URL to an off-chain JSON file with more info.
    pub reference: Option<String>,
    /// Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
    pub reference_hash: Option<String>,
}

/// Offers details on the contract-level metadata.
pub trait MultiTokenMetadataProvider {
    fn mt_metadata(&self) -> MtContractMetadata;
}

impl MtContractMetadata {
    pub fn assert_valid(&self) {
        require!(self.spec == MT_METADATA_SPEC, "Spec is not MT metadata");
        require!(
            self.reference.is_some() == self.reference_hash.is_some(),
            "Reference and reference hash must be present"
        );
        if let Some(reference_hash) = &self.reference_hash {
            require!(reference_hash.len() == 32, "Hash has to be 32 bytes");
        }
    }
}

impl TokenMetadata {
    pub fn assert_valid(&self) {
        require!(self.media.is_some() == self.media_hash.is_some());
        if let Some(media_hash) = &self.media_hash {
            require!(media_hash.len() == 32, "Media hash has to be 32 bytes");
        }

        require!(self.reference.is_some() == self.reference_hash.is_some());
        if let Some(reference_hash) = &self.reference_hash {
            require!(reference_hash.len() == 32, "Reference hash has to be 32 bytes");
        }
    }
}
