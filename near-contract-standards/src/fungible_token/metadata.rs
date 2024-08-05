use near_sdk::json_types::Base64VecU8;
use near_sdk::{ext_contract, near, require};

pub const FT_METADATA_SPEC: &str = "ft-1.0.0";

#[derive(Clone)]
#[near(serializers=[borsh, json])]
pub struct FungibleTokenMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<Base64VecU8>,
    pub decimals: u8,
}

#[ext_contract(ext_ft_metadata)]
pub trait FungibleTokenMetadataProvider {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
}

impl FungibleTokenMetadata {
    pub fn assert_valid(&self) {
        require!(self.spec == FT_METADATA_SPEC);
        require!(self.reference.is_some() == self.reference_hash.is_some());
        if let Some(reference_hash) = &self.reference_hash {
            require!(reference_hash.0.len() == 32, "Hash has to be 32 bytes");
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub mod tests {
    use super::FungibleTokenMetadataProvider;
    use crate::fungible_token::metadata::FT_METADATA_SPEC;

    fn ft_metadata_ok(contract: &mut impl FungibleTokenMetadataProvider) {
        let metadata = contract.ft_metadata();
        assert_eq!(metadata.spec, FT_METADATA_SPEC.to_string());
        assert_eq!(metadata.name, "Example NEAR fungible token");
        assert_eq!(metadata.symbol, "EXAMPLE".to_string());
        assert_eq!(metadata.icon, Some("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E".to_string()));
        assert_eq!(metadata.reference, None);
        assert_eq!(metadata.reference_hash, None);
        assert_eq!(metadata.decimals, 24);
    }

    pub fn test(contract: &mut impl FungibleTokenMetadataProvider) {
        ft_metadata_ok(contract);
    }
}
