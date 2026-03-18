use std::collections::BTreeMap;

use near_account_id::AccountId;
use near_sdk_macros::near;
use serde_with::base64::Base64;

use crate::{GlobalContractId, env};

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[near(inside_nearsdk, serializers = [
    json,
    borsh(use_discriminant = true),
])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum StateInit {
    V1(StateInitV1) = 0,
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

#[cfg_attr(feature = "arbitrary", derive(::arbitrary::Arbitrary))]
#[near(inside_nearsdk, serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "unit-testing", feature = "non-contract-usage")
))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_types::Base58CryptoHash;

    #[test]
    fn test_state_init_json_serialization_externally_tagged() {
        let hash: Base58CryptoHash =
            "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ".parse().unwrap();
        let state_init =
            StateInit::V1(StateInitV1::code(GlobalContractId::CodeHash(hash)));

        let json = serde_json::to_string(&state_init).unwrap();
        // Must use serde's default externally tagged format to match nearcore
        assert!(json.starts_with(r#"{"V1":"#), "expected externally tagged format, got: {json}");

        let deserialized: StateInit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state_init);

        // Old internally tagged format must NOT deserialize
        let old_format = r#"{"version":"v1","code":{"hash":"4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"},"data":{}}"#;
        assert!(
            serde_json::from_str::<StateInit>(old_format).is_err(),
            "old internally tagged format should be rejected"
        );
    }
}
