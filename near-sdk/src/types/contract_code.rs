use near_account_id::AccountId;
use near_sdk_macros::near;

use serde_with::base64::Base64;

use crate::CryptoHash;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractCode {
    Local(CryptoHash),
    Global(GlobalContractId),
}

#[near(inside_nearsdk, serializers = [
    json,
    borsh(use_discriminant = true),
])]
#[serde(untagged)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum GlobalContractId {
    CodeHash(#[serde_as(as = "Base64")] CryptoHash) = 0,
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

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
const _: () = {
    use near_primitives_core::global_contract::GlobalContractIdentifier;

    impl From<GlobalContractIdentifier> for GlobalContractId {
        fn from(value: GlobalContractIdentifier) -> Self {
            match value {
                GlobalContractIdentifier::CodeHash(code_hash) => Self::CodeHash(code_hash.0),
                GlobalContractIdentifier::AccountId(account_id) => Self::AccountId(account_id),
            }
        }
    }
};
