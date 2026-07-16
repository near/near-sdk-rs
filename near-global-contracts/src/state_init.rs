use crate::GlobalContractId;
use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde_with::base64::Base64;

/// Initial on-chain state used to derive a [deterministic account].
///
/// A deterministic account ([NEP-616]) lives at an address computed from its own initial
/// state instead of one chosen up front, so anyone who knows the state can work out the
/// address ahead of time. This type is that state; pass it to
/// [`StateInit::derive_account_id`] to get the resulting [`AccountId`].
///
/// It is versioned so the layout can change later without breaking existing data.
/// [`V1`](StateInit::V1) is the only version today.
///
/// [deterministic account]: https://github.com/near/NEPs/pull/616
/// [NEP-616]: https://github.com/near/NEPs/pull/616
/// [`AccountId`]: near_account_id::AccountId
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[repr(u8)]
pub enum StateInit {
    /// Version 1: see [`StateInitV1`].
    V1(StateInitV1) = 0,
}

impl StateInit {
    /// Derives [`AccountId`](near_account_id::AccountId) deterministically, according to NEP-616.
    ///
    /// # Availability
    ///
    /// This method is only compiled when:
    /// - the `borsh` feature is enabled (needed to serialize the input for hashing), AND
    /// - one of the following is true:
    ///   - `--cfg near` is set (on-chain contract build; `cargo-near` sets this automatically) —
    ///     routes through the `keccak256` host function via the `near-sdk-env` crate.
    ///   - the `digest` feature is enabled (off-chain or non-NEAR wasm build) — uses pure-Rust
    ///     `sha3::Keccak256`.
    ///
    /// If you see "no method named `derive_account_id`" on a `StateInit`, add the `digest`
    /// feature to your `near-global-contracts` dependency.
    #[inline]
    #[cfg(feature = "borsh")]
    pub fn derive_account_id(&self) -> near_account_id::AccountId {
        let hash: [u8; 32];

        #[cfg(any(near, feature = "__near-sdk-unit-testing"))]
        {
            let serialized = borsh::to_vec(self).unwrap_or_else(|_| unreachable!());
            // SAFETY: keccak256 hash will always generate 32 bytes
            hash = near_sdk_env::keccak256_array(&serialized);
        }
        #[cfg(not(any(near, feature = "__near-sdk-unit-testing")))]
        {
            use digest_io::IoWrapper;
            use sha3::Digest;

            let mut hasher = IoWrapper(sha3::Keccak256::new());
            borsh::to_writer(&mut hasher, self).unwrap_or_else(|_| unreachable!());
            // SAFETY: keccak256 hash will always generate 32 bytes
            hash = hasher.0.finalize().into();
        }

        // SAFETY: 20 bytes-long hash will produce 40 hex chars.
        // "0s" + 40 hex chars = 42 chars, which is within `AccountId` length bounds (2-64).
        // `hex::encode` always produces valid [0-9a-f] characters, hence, we can construct
        // AccountId without validation
        #[allow(deprecated)]
        near_account_id::AccountId::new_unvalidated(format!(
            "0s{}",
            // SAFETY: keccak256 hash will always generate 32 bytes; [12..32] is exactly 20 bytes,
            // matching 20 byte-long hash requirement to fit the near's `AccountId` length bounds
            hex::encode(&hash[12..32])
        ))
    }
}

/// Version 1 of the [`StateInit`] payload.
///
/// Holds the global contract to run and the account's initial storage entries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(
    feature = "serde",
    cfg_eval::cfg_eval,
    serde_with::serde_as,
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(
    feature = "schemars-v0_8",
    derive(::schemars_v0_8::JsonSchema),
    schemars(crate = "::schemars_v0_8")
)]
#[cfg_attr(feature = "abi", derive(borsh::BorshSchema))]
pub struct StateInitV1 {
    /// The global contract whose code the derived account will run.
    pub code: GlobalContractId,
    /// Key/value entries written to the account's trie storage at creation.
    #[cfg_attr(feature = "serde", serde_as(as = "BTreeMap<Base64, Base64>"))]
    pub data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl StateInitV1 {
    /// Creates a payload that runs `code` with no initial storage.
    ///
    /// Chain [`with_data_entry`](Self::with_data_entry) to add storage entries.
    #[inline]
    pub fn code(code: impl Into<GlobalContractId>) -> Self {
        Self { code: code.into(), data: BTreeMap::new() }
    }

    /// Adds one key/value entry to the initial storage, returning `self` so calls can be
    /// chained.
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

#[cfg(feature = "near-primitives-interop")]
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
mod test {
    use crate::StateInit;

    #[test]
    #[cfg(all(feature = "serde", feature = "borsh"))]
    fn test_state_init_account_id_derivation() {
        use near_account_id::AccountId;

        let test_data: Vec<(StateInit, AccountId)> = vec![
            (
                serde_json::from_value(serde_json::json!({
                    "V1": {
                        "code": { "hash": "J86LNmZE9nHAxRqUYBZ64iCQYfeacMJhNqvb8WQmpZPE"},
                        "data": { "AAEC": "AwQF" },
                    }
                }))
                .unwrap(),
                AccountId::try_from("0s48ddf87e648de3a52783ee9640e618234cadb18f").unwrap(),
            ),
            (
                serde_json::from_value(serde_json::json!({
                    "V1": {
                        "code": { "account_id": "alice.near"},
                        "data": { "AAEC": "AwQF" },
                    }
                }))
                .unwrap(),
                AccountId::try_from("0sf4d27a587616342eb45b8d785addbe6790695a2e").unwrap(),
            ),
        ];

        for (state, expected_result) in test_data {
            assert!(state.derive_account_id() == expected_result)
        }
    }
}
