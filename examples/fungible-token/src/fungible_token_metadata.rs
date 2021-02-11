use super::*;
use near_sdk::serde::Serialize;

#[derive(Serialize, BorshDeserialize, BorshSerialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FungibleTokenMetadata {
    pub version: String,
    pub name: String,
    pub symbol: String,
    pub reference: String,
    pub decimals: u8,
}

pub trait FungibleTokenMetadataProvider {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.clone()
    }
}
