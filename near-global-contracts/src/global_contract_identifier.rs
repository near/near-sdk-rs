use std::borrow::Cow;

use near_account_id::{AccountId, AccountIdRef};

use near_crypto_hash::CryptoHash;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountContract {
    None,
    Local(CryptoHash),
    Global(CryptoHash),
    GlobalByAccount(AccountId),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    borsh(use_discriminant = true)
)]
#[cfg_attr(
    feature = "schemars-v0_8",
    derive(::schemars_v0_8::JsonSchema),
    schemars(crate = "::schemars_v0_8")
)]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum GlobalContractId {
    #[cfg_attr(feature = "serde", serde(rename = "hash"))]
    CodeHash(
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base58::Base58"))] CryptoHash,
    ) = 0,
    #[cfg_attr(feature = "serde", serde(rename = "account_id"))]
    AccountId(AccountId) = 1,
}

impl GlobalContractId {
    /// Derive (immutable) global contract id as hash of given code
    #[cfg(feature = "digest")]
    #[inline]
    pub fn hash_of(code: impl AsRef<[u8]>) -> Self {
        use near_digest::{Digest, sha2::Sha256};

        Self::CodeHash(Sha256::digest(code).into())
    }
}

impl From<CryptoHash> for GlobalContractId {
    #[inline]
    fn from(hash: CryptoHash) -> Self {
        Self::CodeHash(hash)
    }
}

impl From<&CryptoHash> for GlobalContractId {
    #[inline]
    fn from(hash: &CryptoHash) -> Self {
        Self::CodeHash(*hash)
    }
}

impl From<AccountId> for GlobalContractId {
    #[inline]
    fn from(account_id: AccountId) -> Self {
        Self::AccountId(account_id)
    }
}

impl From<&AccountId> for GlobalContractId {
    #[inline]
    fn from(account_id: &AccountId) -> Self {
        Self::AccountId(account_id.clone())
    }
}

impl From<&AccountIdRef> for GlobalContractId {
    #[inline]
    fn from(account_id: &AccountIdRef) -> Self {
        Self::AccountId(account_id.to_owned())
    }
}

impl From<Cow<'_, AccountIdRef>> for GlobalContractId {
    #[inline]
    fn from(account_id: Cow<'_, AccountIdRef>) -> Self {
        Self::AccountId(account_id.into_owned())
    }
}

impl From<&GlobalContractId> for GlobalContractId {
    #[inline]
    fn from(value: &GlobalContractId) -> Self {
        value.clone()
    }
}

#[cfg(feature = "near-primitives-interop")]
const _: () = {
    use near_primitives_core::{
        account::AccountContract as NearAccountContract,
        global_contract::GlobalContractIdentifier as NearGlobalContractIdentifier,
        hash::CryptoHash,
    };

    impl From<NearAccountContract> for AccountContract {
        fn from(value: NearAccountContract) -> Self {
            match value {
                NearAccountContract::None => Self::None,
                NearAccountContract::Local(contract) => Self::Local(contract.0),
                NearAccountContract::Global(contract) => Self::Global(contract.0),
                NearAccountContract::GlobalByAccount(account_id) => {
                    Self::GlobalByAccount(account_id)
                }
            }
        }
    }

    impl From<AccountContract> for NearAccountContract {
        fn from(value: AccountContract) -> Self {
            match value {
                AccountContract::None => Self::None,
                AccountContract::Local(contract) => Self::Local(CryptoHash(contract)),
                AccountContract::Global(contract) => Self::Global(CryptoHash(contract)),
                AccountContract::GlobalByAccount(account_id) => Self::GlobalByAccount(account_id),
            }
        }
    }

    impl From<NearGlobalContractIdentifier> for GlobalContractId {
        fn from(value: NearGlobalContractIdentifier) -> Self {
            match value {
                NearGlobalContractIdentifier::CodeHash(code_hash) => Self::CodeHash(code_hash.0),
                NearGlobalContractIdentifier::AccountId(account_id) => Self::AccountId(account_id),
            }
        }
    }

    impl From<GlobalContractId> for NearGlobalContractIdentifier {
        fn from(value: GlobalContractId) -> Self {
            match value {
                GlobalContractId::CodeHash(code_hash) => Self::CodeHash(CryptoHash(code_hash)),
                GlobalContractId::AccountId(account_id) => Self::AccountId(account_id),
            }
        }
    }
};

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn test_global_contract_id_json_serialization_code_hash() {
        // The code hash is a raw `CryptoHash` but (de)serializes as a Base58 string.
        let json = r#"{"hash":"4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"}"#;

        let id: GlobalContractId = serde_json::from_str(json).unwrap();
        assert!(matches!(id, GlobalContractId::CodeHash(_)));

        // Round-trips back to the same Base58 string, and deserialization is stable.
        assert_eq!(serde_json::to_string(&id).unwrap(), json);
        assert_eq!(serde_json::from_str::<GlobalContractId>(json).unwrap(), id);
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
