use crate::GlobalContractId;
use std::collections::BTreeMap;

#[cfg(any(near, feature = "serde"))]
use serde_with::base64::Base64;

#[cfg(feature = "schemars-v0_8")]
use schemars_v0_8 as schemars;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(any(near, feature = "serde"), derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    any(near, feature = "borsh"),
    derive(borsh::BorshSerialize, borsh::BorshDeserialize),
    borsh(use_discriminant = true)
)]
#[cfg_attr(feature = "schemars-v0_8", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
#[repr(u8)]
pub enum StateInit {
    V1(StateInitV1) = 0,
}

impl StateInit {
    /// Derives [`AccountId`](near_account_id::AccountId) deterministically, according to NEP-616.
    #[inline]
    #[cfg(all(feature = "borsh", any(near, feature = "digest")))]
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        let hash: [u8; 20];

        #[cfg(near)]
        {
            let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
            // SAFETY: keccak256 hash will always generate 32 bytes; [12..32] is exactly
            // 20 bytes, matching [u8; 20]
            hash = near_env::keccak256_array(&serialized)[12..32]
                .try_into()
                .unwrap_or_else(|_| unreachable!());
        }
        #[cfg(all(feature = "digest", not(near)))]
        {
            use sha3::Digest;

            let mut hasher = sha3::Keccak256::new();
            borsh::to_writer(&mut hasher, self).unwrap_or_else(|_| unreachable!());
            // SAFETY: keccak256 hash will always generate 32 bytes; [12..32] is exactly
            // 20 bytes, matching [u8; 20]
            hash = hasher.finalize()[12..32].try_into().unwrap_or_else(|_| unreachable!());
        }

        // SAFETY: 20 bytes-long hash will produce 40 hex chars.
        // "0s" + 40 hex chars = 42 chars, which is within `AccountId` length bounds (2-64).
        // `hex::encode` always produces valid [0-9a-f] characters, hence, we can construct
        // AccountId without validation
        #[allow(deprecated)]
        near_account_id::AccountId::new_unvalidated(format!("0s{}", hex::encode(hash)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    any(near, feature = "serde"),
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(any(near, feature = "borsh"), derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "schemars-v0_8", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
pub struct StateInitV1 {
    pub code: GlobalContractId,
    #[cfg_attr(any(near, feature = "serde"), serde_as(as = "BTreeMap<Base64, Base64>"))]
    #[cfg_attr(feature = "schemars-v0_8", schemars(with = "BTreeMap<String, String>"))]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl StateInitV1 {
    #[inline]
    pub const fn code(code: GlobalContractId) -> Self {
        Self { code, data: BTreeMap::new() }
    }

    #[inline]
    pub fn with_data_entry(mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        self.data.insert(key.into(), value.into());
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
