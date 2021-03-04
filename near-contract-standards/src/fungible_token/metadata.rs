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
    pub reference: Option<String>,
    pub reference_hash: Option<Base64VecU8>,
    pub decimals: u8,
}

pub trait FungibleTokenMetadataProvider {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
}

pub fn are_valid_metadata_params(name: Option<String>, symbol: Option<String>, icon: Option<String>, reference: Option<String>, reference_hash: Option<Base64VecU8>, decimals: Option<u8>) -> bool {
    // If any metadata params are specified, all required params must also be specified.
    if name.is_some() || symbol.is_some() || icon.is_some() || reference.is_some() || reference_hash.is_some() || decimals.is_some() {
        name.is_some() && symbol.is_some() && decimals.is_some()
    } else {
        // This contract requires metadata
        false
    }
}
