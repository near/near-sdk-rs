use crate::GlobalContractId;
use std::collections::BTreeMap;

#[cfg(feature = "serde")]
mod serde_impl;

#[cfg(feature = "borsh")]
use near_account_id::AccountId;

#[cfg(feature = "abi")]
mod schemars_impl;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "borsh", borsh(use_discriminant = true))]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
#[cfg_attr(feature = "abi", derive(schemars::JsonSchema))]
#[repr(u8)]
pub enum StateInit {
    V1(StateInitV1) = 0,
}

impl StateInit {
    /// Derives [`AccountId`] deterministically, according to NEP-616.
    #[inline]
    #[cfg(feature = "borsh")]
    pub fn derive_account_id(&self) -> AccountId {
        #[cfg(feature = "near-contracts")]
        {
            let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
            format!("0s{}", hex::encode(&near_env::keccak256_array(&serialized)[12..32]))
                .parse()
                .unwrap_or_else(|_| unreachable!())
        }
        #[cfg(not(feature = "near-contracts"))]
        {
            use sha3::Digest;

            let mut hasher = sha3::Keccak256::new();
            borsh::to_writer(&mut hasher, self).unwrap_or_else(|_| unreachable!());
            let hash = hasher.finalize();
            format!("0s{}", hex::encode(&hash[12..32])).parse().unwrap_or_else(|_| unreachable!())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
pub struct StateInitV1 {
    pub code: GlobalContractId,
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

#[cfg(all(not(target_arch = "wasm32"), feature = "near-primitives-interop"))]
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

    impl From<StateInit> for DeterministicAccountStateInit {
        fn from(value: StateInit) -> Self {
            match value {
                StateInit::V1(state_init) => Self::V1(state_init.into()),
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

    impl From<StateInitV1> for DeterministicAccountStateInitV1 {
        fn from(StateInitV1 { code, data }: StateInitV1) -> Self {
            Self { code: code.into(), data }
        }
    }
};
