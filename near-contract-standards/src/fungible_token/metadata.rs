use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::Serialize;

pub const FT_METADATA_SPEC: &str = "ftm-1.0.0";

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FungibleTokenMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: String,
    pub reference_hash: Base64VecU8,
    pub decimals: u8,
}

pub trait FungibleTokenMetadataProvider {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
}
