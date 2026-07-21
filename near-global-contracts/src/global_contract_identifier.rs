use near_account_id::AccountId;

use near_crypto_hash::CryptoHash;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountContract {
    None,
    Local(CryptoHash),
    Global(CryptoHash),
    GlobalByAccount(AccountId),
}

/// Identifies which [global contract] an account should run.
///
/// Global contracts let contract code be deployed once and shared across the whole
/// network, so many accounts can run the same code without each storing its own copy.
/// A global contract is referenced in one of two ways: by hash of its code or by `AccountId` that deployed it.
///
/// [global contract]: https://github.com/near/NEPs/pull/591
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
    /// Reference the contract by the hash of its code.
    ///
    /// This pins that exact bytecode: the reference never changes, even if the account
    /// that originally deployed the code later deploys something else. Use it when you
    /// want an immutable dependency.
    #[cfg_attr(feature = "serde", serde(rename = "hash"))]
    CodeHash(
        #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::base58::Base58"))] CryptoHash,
    ) = 0,
    /// Reference the contract by the account that deployed it globally.
    ///
    /// This follows whatever code that account currently has deployed as a global
    /// contract, so the referenced code changes if the deployer upgrades it. Use it when
    /// you want to track the deployer's latest version.
    #[cfg_attr(feature = "serde", serde(rename = "account_id"))]
    AccountId(AccountId) = 1,
}

impl From<CryptoHash> for GlobalContractId {
    #[inline]
    fn from(hash: CryptoHash) -> Self {
        Self::CodeHash(hash)
    }
}

impl From<AccountId> for GlobalContractId {
    #[inline]
    fn from(account_id: AccountId) -> Self {
        Self::AccountId(account_id)
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
