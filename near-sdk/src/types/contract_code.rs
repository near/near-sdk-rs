use near_account_id::AccountId;
use near_sdk_macros::near;

use crate::json_types::Base58CryptoHash;
use crate::CryptoHash;

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum AccountContract {
    None,
    Local(Base58CryptoHash),
    Global(Base58CryptoHash),
    GlobalByAccount(AccountId),
}

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[near(inside_nearsdk, serializers = [
    json,
    borsh(use_discriminant = true),
])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum GlobalContractId {
    #[serde(rename = "hash")]
    CodeHash(Base58CryptoHash) = 0,
    #[serde(rename = "account_id")]
    AccountId(AccountId) = 1,
}

impl From<CryptoHash> for GlobalContractId {
    #[inline]
    fn from(hash: CryptoHash) -> Self {
        Self::CodeHash(hash.into())
    }
}

impl From<Base58CryptoHash> for GlobalContractId {
    #[inline]
    fn from(hash: Base58CryptoHash) -> Self {
        Self::CodeHash(hash)
    }
}

impl From<AccountId> for GlobalContractId {
    #[inline]
    fn from(account_id: AccountId) -> Self {
        Self::AccountId(account_id)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
const _: () = {
    use near_primitives_core::{
        account::AccountContract as NearAccountContract,
        global_contract::GlobalContractIdentifier as NearGlobalContractIdentifier,
    };

    impl From<NearAccountContract> for AccountContract {
        fn from(value: NearAccountContract) -> Self {
            match value {
                NearAccountContract::None => Self::None,
                NearAccountContract::Local(contract) => Self::Local(contract.0.into()),
                NearAccountContract::Global(contract) => Self::Global(contract.0.into()),
                NearAccountContract::GlobalByAccount(account_id) => {
                    Self::GlobalByAccount(account_id)
                }
            }
        }
    }

    impl From<NearGlobalContractIdentifier> for GlobalContractId {
        fn from(value: NearGlobalContractIdentifier) -> Self {
            match value {
                NearGlobalContractIdentifier::CodeHash(code_hash) => {
                    Self::CodeHash(code_hash.0.into())
                }
                NearGlobalContractIdentifier::AccountId(account_id) => Self::AccountId(account_id),
            }
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_types::Base58CryptoHash;

    #[test]
    fn test_global_contract_id_json_serialization_code_hash() {
        let hash: Base58CryptoHash =
            "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ".parse().unwrap();
        let id = GlobalContractId::CodeHash(hash);

        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#"{"hash":"4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"}"#);

        let deserialized: GlobalContractId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    #[test]
    fn test_global_contract_id_json_serialization_account_id() {
        let account_id: AccountId = "alice.near".parse().unwrap();
        let id = GlobalContractId::AccountId(account_id.clone());

        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#"{"account_id":"alice.near"}"#);

        let deserialized: GlobalContractId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }
}
