use near_account_id::AccountId;
use near_sdk_macros::near;

use crate::CryptoHash;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum AccountContract {
    None,
    Local(CryptoHash),
    Global(CryptoHash),
    GlobalByAccount(AccountId),
}

#[near(inside_nearsdk, serializers = [
    json,
    borsh(use_discriminant = true),
])]
#[serde(untagged)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum GlobalContractIdentifier {
    CodeHash(
        #[serde_as(as = "::serde_with::base64::Base64")]
        #[cfg_attr(feature = "abi", schemars(with = "String"))]
        CryptoHash,
    ) = 0,
    AccountId(AccountId) = 1,
}

impl From<CryptoHash> for GlobalContractIdentifier {
    #[inline]
    fn from(hash: CryptoHash) -> Self {
        Self::CodeHash(hash)
    }
}

impl From<AccountId> for GlobalContractIdentifier {
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
                NearAccountContract::Local(contract) => Self::Local(contract.0),
                NearAccountContract::Global(contract) => Self::Global(contract.0),
                NearAccountContract::GlobalByAccount(account_id) => {
                    Self::GlobalByAccount(account_id)
                }
            }
        }
    }

    impl From<NearGlobalContractIdentifier> for GlobalContractIdentifier {
        fn from(value: NearGlobalContractIdentifier) -> Self {
            match value {
                NearGlobalContractIdentifier::CodeHash(code_hash) => Self::CodeHash(code_hash.0),
                NearGlobalContractIdentifier::AccountId(account_id) => Self::AccountId(account_id),
            }
        }
    }
};
