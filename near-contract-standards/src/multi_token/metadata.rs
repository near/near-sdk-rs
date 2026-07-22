use near_sdk::json_types::Base64VecU8;
use near_sdk::{ext_contract, near, require};

/// This spec can be treated like a version of the standard.
pub const MT_METADATA_SPEC: &str = "mt-1.0.0";

/// Metadata for the MT contract itself.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers=[borsh, json])]
pub struct MTContractMetadata {
    pub spec: String, // required, essentially a version like "mt-1.0.0"
    pub name: String, // required, ex. "Zoink's Digital Sword Collection"
}

/// Base token metadata that applies to token types.
#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MTBaseTokenMetadata {
    pub name: String,              // required, ex. "Silver Swords" or "Metaverse 3"
    pub id: String,                // required, a unique identifier for the metadata
    pub symbol: Option<String>,    // ex. "MOCHI"
    pub icon: Option<String>,      // Data URL
    pub decimals: Option<String>,  // number of decimals for the token useful for FT related tokens
    pub base_uri: Option<String>, // Centralized gateway known to have reliable access to decentralized storage assets referenced by `reference` or `media` URLs
    pub reference: Option<String>, // URL to a JSON file with more info
    pub copies: Option<u64>, // number of copies of this set of metadata in existence when token was minted.
    pub reference_hash: Option<Base64VecU8>, // Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
}

/// Token-specific metadata.
#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MTTokenMetadata {
    pub title: Option<String>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
    pub description: Option<String>, // free-form description
    pub media: Option<String>, // URL to associated media, preferably to decentralized, content-addressed storage
    pub media_hash: Option<Base64VecU8>, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
    pub issued_at: Option<String>, // When token was issued or minted, Unix epoch in milliseconds
    pub expires_at: Option<String>, // When token expires, Unix epoch in milliseconds
    pub starts_at: Option<String>, // When token starts being valid, Unix epoch in milliseconds
    pub updated_at: Option<String>, // When token was last updated, Unix epoch in milliseconds
    pub extra: Option<String>, // Anything extra the MT wants to store on-chain. Can be stringified JSON.
    pub reference: Option<String>, // URL to an off-chain JSON file with more info.
    pub reference_hash: Option<Base64VecU8>, // Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
}

/// Combined metadata for a token.
#[near(serializers=[borsh, json])]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MTTokenMetadataAll {
    pub base: MTBaseTokenMetadata,
    pub token: MTTokenMetadata,
}

/// Offers details on the contract-level metadata.
#[ext_contract(ext_mt_metadata_provider)]
pub trait MultiTokenMetadataProvider {
    /// Returns the top-level contract level metadata
    fn mt_metadata_contract(&self) -> MTContractMetadata;

    /// Returns combined base and token metadata for given token IDs
    fn mt_metadata_token_all(&self, token_ids: Vec<String>) -> Vec<MTTokenMetadataAll>;

    /// Returns token-specific metadata for given token IDs
    fn mt_metadata_token_by_token_id(&self, token_ids: Vec<String>) -> Vec<MTTokenMetadata>;

    /// Returns base metadata for given token IDs
    fn mt_metadata_base_by_token_id(&self, token_ids: Vec<String>) -> Vec<MTBaseTokenMetadata>;

    /// Returns base metadata for given base metadata IDs
    fn mt_metadata_base_by_metadata_id(
        &self,
        base_metadata_ids: Vec<String>,
    ) -> Vec<MTBaseTokenMetadata>;
}

impl MTContractMetadata {
    pub fn assert_valid(&self) {
        require!(self.spec == MT_METADATA_SPEC, "Spec is not MT metadata");
    }
}

impl MTBaseTokenMetadata {
    pub fn assert_valid(&self) {
        require!(
            self.reference.is_some() == self.reference_hash.is_some(),
            "Reference and reference hash must be present together"
        );
        if let Some(reference_hash) = &self.reference_hash {
            require!(reference_hash.0.len() == 32, "Hash has to be 32 bytes");
        }
    }
}

impl MTTokenMetadata {
    pub fn assert_valid(&self) {
        require!(
            self.media.is_some() == self.media_hash.is_some(),
            "Media and media hash must be present together"
        );
        if let Some(media_hash) = &self.media_hash {
            require!(media_hash.0.len() == 32, "Media hash has to be 32 bytes");
        }

        require!(
            self.reference.is_some() == self.reference_hash.is_some(),
            "Reference and reference hash must be present together"
        );
        if let Some(reference_hash) = &self.reference_hash {
            require!(reference_hash.0.len() == 32, "Reference hash has to be 32 bytes");
        }
    }
}
