use std::collections::BTreeMap;

use near_account_id::AccountId;
use near_sdk_macros::near;
use serde_with::{base64::Base64, serde_as};

use crate::{env, GlobalContractId};

#[near(inside_nearsdk, serializers = [json, borsh])]
#[serde(tag = "version")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateInit {
    V1(
        // `self::` fixes #[derive(BorshSchema)] infinite recursion
        // due to naming collision
        self::StateInitV1,
    ),
}

impl StateInit {
    /// Derives [`AccountId`] deterministically, according to NEP-616.
    #[inline]
    pub fn derive_account_id(&self) -> AccountId {
        let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
        format!("0s{}", hex::encode(&env::keccak256_array(&serialized)[12..32]))
            .parse()
            .unwrap_or_else(|_| unreachable!())
    }
}

#[near(inside_nearsdk, serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInitV1 {
    pub code: GlobalContractId,
    #[serde_as(as = "BTreeMap<Base64, Base64>")]
    #[cfg_attr(feature = "abi", schemars(with = "BTreeMap<String, String>"))]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl StateInitV1 {
    #[inline]
    pub const fn code(code: GlobalContractId) -> Self {
        Self { code, data: BTreeMap::new() }
    }

    #[inline]
    pub fn with_data_entry(mut self, key: Vec<u8>, value: Vec<u8>) -> Self {
        self.data.insert(key, value);
        self
    }
}

impl From<StateInitV1> for StateInit {
    #[inline]
    fn from(state_init: StateInitV1) -> Self {
        Self::V1(state_init)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
const _: () = {
    use near_primitives_core::deterministic_account_id::{
        DeterministicAccountStateInit, DeterministicAccountStateInitV1,
    };

    impl From<DeterministicAccountStateInit> for StateInit {
        fn from(value: DeterministicAccountStateInit) -> Self {
            match value {
                DeterministicAccountStateInit::V1(state_init) => Self::V1(state_init.into()),
            }
        }
    }

    impl From<DeterministicAccountStateInitV1> for StateInitV1 {
        fn from(
            DeterministicAccountStateInitV1 { code, data }: DeterministicAccountStateInitV1,
        ) -> Self {
            Self { code: code.into(), data }
        }
    }
};
