use std::{
    borrow::Cow,
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use borsh::{io, BorshDeserialize, BorshSerialize};
use near_account_id::AccountId;
use near_sdk_macros::near;
use near_token::NearToken;
use serde_with::base64::Base64;

use crate::{env, CryptoHash, StorageUsage};

/// Initialization state for non-existing contract with deterministic
/// account id, according to NEP-616.
#[near(inside_nearsdk, serializers=[borsh, json])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInit {
    /// Code to deploy
    pub code: ContractCode,
    /// Optional key/value pairs to populate to storage on first initialization
    pub data: ContractStorage,
}

impl StateInit {
    /// Create new [`StateInit`] with given code and no data
    #[inline]
    pub fn code(code: impl Into<ContractCode>) -> Self {
        Self { code: code.into(), data: ContractStorage::new() }
    }

    /// Set given data
    #[inline]
    pub fn data(mut self, data: ContractStorage) -> Self {
        self.data = data;
        self
    }

    /// Derives [`AccountId`] deterministically, according to NEP-616.
    /// See [`LazyStateInit::derive_account_id`].
    #[inline]
    pub fn derive_account_id(&self) -> AccountId {
        self.lazy_serialized().derive_account_id()
    }

    /// Estimate amount of NEAR required to cover the storage costs for
    /// deploying and initializing the contract.
    ///
    /// Note that this estimation is based on current runtime storage config,
    /// so if these values change while in-flight, then the estimated amount
    /// can be insufficient to cover for updated protocol config values.
    /// In this case, one can just retry the operation and it would succeed.
    #[inline]
    pub fn storage_cost(&self) -> NearToken {
        self.storage_usage()
            .and_then(|s| env::storage_byte_cost().checked_mul(s.into()))
            .unwrap_or_else(|| env::panic_str("too big"))
    }

    /// See NEP-591: https://github.com/near/NEPs/blob/master/neps/nep-0591.md#costs
    pub(crate) fn storage_usage(&self) -> Option<StorageUsage> {
        // `num_bytes_account` is required for every account on creation:
        // https://github.com/near/nearcore/blob/685f92e3b9efafc966c9dafcb7815f172d4bb557/runtime/runtime/src/actions.rs#L468
        env::storage_num_bytes_account()
            .checked_add(self.code.storage_usage())?
            .checked_add(self.data.storage_usage()?)
    }

    #[inline]
    pub const fn lazy(self) -> LazyStateInit {
        LazyStateInit(LazyStateInitInner::StateInit(self))
    }

    pub fn lazy_serialized(&self) -> LazyStateInit {
        LazyStateInit(LazyStateInitInner::Serialized(
            borsh::to_vec(self).unwrap_or_else(|_| unreachable!()),
        ))
    }
}

/// Code to deploy for non-existing contract
#[near(inside_nearsdk, serializers=[borsh, json])]
#[serde(tag = "location", content = "data", rename_all = "snake_case")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractCode {
    /// Reference global contract's code by its hash
    GlobalCodeHash(#[serde_as(as = "Base64")] CryptoHash),

    /// Reference global contract's code by its [`AccountId`]
    GlobalAccountId(AccountId),
}

impl ContractCode {
    pub(crate) fn storage_usage(&self) -> StorageUsage {
        // Global contract identifier, see NEP-591:
        // https://github.com/near/NEPs/blob/master/neps/nep-0591.md#costs
        // Here is nearcore implementation:
        // https://github.com/near/nearcore/blob/685f92e3b9efafc966c9dafcb7815f172d4bb557/core/primitives-core/src/account.rs#L123-L128
        match self {
            Self::GlobalCodeHash(hash) => hash.len() as StorageUsage,
            Self::GlobalAccountId(account_id) => account_id.len() as StorageUsage,
        }
    }
}

impl From<CryptoHash> for ContractCode {
    #[inline]
    fn from(hash: CryptoHash) -> Self {
        Self::GlobalCodeHash(hash)
    }
}

impl From<AccountId> for ContractCode {
    #[inline]
    fn from(account_id: AccountId) -> Self {
        Self::GlobalAccountId(account_id)
    }
}

/// Key-value storage of a contract
#[near(inside_nearsdk, serializers=[borsh, json])]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct ContractStorage(
    #[serde_as(as = "BTreeMap<Base64, Base64>")] pub BTreeMap<Vec<u8>, Vec<u8>>,
);

impl ContractStorage {
    #[inline]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn borsh<K, V>(mut self, key: K, value: V) -> Self
    where
        K: BorshSerialize,
        V: BorshSerialize,
    {
        self.0.insert(
            borsh::to_vec(&key).unwrap_or_else(|_| unreachable!()),
            borsh::to_vec(&value).unwrap_or_else(|_| unreachable!()),
        );
        self
    }

    pub(crate) fn storage_usage(&self) -> Option<StorageUsage> {
        let num_extra_bytes_record = env::storage_num_extra_bytes_record();
        self.iter().try_fold(0u64, |storage_usage, (key, value)| {
            // key.len() + value.len() + num_extra_bytes_record:
            // https://github.com/near/nearcore/blob/1c2903faeb47fdaf40d5d140cec78aa9bab018ae/runtime/near-vm-runner/src/logic/logic.rs#L3311-L3346
            storage_usage
                .checked_add(key.len().try_into().ok()?)?
                .checked_add(value.len().try_into().ok()?)?
                .checked_add(num_extra_bytes_record)
        })
    }
}

impl Deref for ContractStorage {
    type Target = BTreeMap<Vec<u8>, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ContractStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Maybe serialized [`StateInit`]
#[near(inside_nearsdk, serializers=[borsh, json])]
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct LazyStateInit(LazyStateInitInner);

#[near(inside_nearsdk, serializers=[json])]
#[derive(Debug, Clone)]
#[serde(untagged)]
enum LazyStateInitInner {
    StateInit(StateInit),
    Serialized(#[serde_as(as = "Base64")] Vec<u8>),
}

impl LazyStateInit {
    /// Derives [`AccountId`] deterministically, according to NEP-616.
    ///
    /// The schema looks like: `"0s" .. hex(keccak256(state_init)[12..32])`.
    ///
    /// Such schema is backwards-compatible with existing [`AccountId`] types.
    /// It looks similar to existing implicit Eth addresses but we
    /// intentionally use a different prefix, so it's possible to distinguish
    /// between different kinds of accounts in runtime and apply different
    /// rules for estimating gas and storage costs.
    #[inline]
    pub fn derive_account_id(&self) -> AccountId {
        format!("0s{}", hex::encode(&env::keccak256_array(&self.serialize())[12..32]))
            .parse()
            .unwrap_or_else(|_| unreachable!())
    }

    #[inline]
    pub fn serialize(&self) -> Cow<'_, [u8]> {
        match &self.0 {
            LazyStateInitInner::StateInit(state_init) => {
                Cow::Owned(borsh::to_vec(state_init).unwrap_or_else(|_| unreachable!()))
            }
            LazyStateInitInner::Serialized(data) => Cow::Borrowed(data),
        }
    }

    #[inline]
    pub fn deserialize(self) -> io::Result<StateInit> {
        match self.0 {
            LazyStateInitInner::StateInit(state_init) => Ok(state_init),
            LazyStateInitInner::Serialized(data) => borsh::from_slice(&data),
        }
    }
}

impl From<StateInit> for LazyStateInit {
    fn from(state_init: StateInit) -> Self {
        state_init.lazy()
    }
}

impl PartialEq for LazyStateInit {
    fn eq(&self, other: &Self) -> bool {
        self.serialize() == other.serialize()
    }
}

impl Eq for LazyStateInit {}

impl BorshSerialize for LazyStateInitInner {
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            Self::StateInit(state_init) => BorshSerialize::serialize(state_init, writer),
            Self::Serialized(data) => writer.write_all(data),
        }
    }
}

impl BorshDeserialize for LazyStateInitInner {
    fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        BorshDeserialize::deserialize_reader(reader).map(Self::StateInit)
    }
}

#[cfg(feature = "abi")]
const _: () = {
    use borsh::{
        schema::{Declaration, Definition},
        BorshSchema,
    };

    impl BorshSchema for LazyStateInitInner {
        fn add_definitions_recursively(definitions: &mut BTreeMap<Declaration, Definition>) {
            <StateInit as BorshSchema>::add_definitions_recursively(definitions);
        }

        fn declaration() -> Declaration {
            <StateInit as BorshSchema>::declaration()
        }
    }
};

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, sync::LazyLock};

    use serde::{de::DeserializeOwned, Serialize};

    use super::*;

    static STATE_INIT: LazyLock<StateInit> = LazyLock::new(|| {
        StateInit::code(ContractCode::GlobalAccountId("global.near".parse().unwrap()))
            .data(ContractStorage::new().borsh("key", "value"))
    });

    #[test]
    fn state_init_derive_account_id() {
        let derived = STATE_INIT.derive_account_id();
        println!("derived: {derived}");
        assert!(derived.is_top_level());
    }

    #[test]
    fn state_init_derive_account_id2() {
        let derived1 = STATE_INIT.derive_account_id();

        let mut state_init2 = STATE_INIT.clone();
        state_init2.data = state_init2.data.borsh("key2", "value2");
        let derived2 = state_init2.derive_account_id();

        assert_ne!(derived1, derived2);
    }

    #[test]
    fn state_init_storage_usage_and_cost() {
        println!(
            "storage usage: {} bytes, storage cost: {}",
            STATE_INIT.storage_usage().unwrap(),
            STATE_INIT.storage_cost()
        );
    }

    #[test]
    fn state_init_borsh_roundtrip() {
        let deserialized = assert_borsh_roundtrip(&*STATE_INIT);

        assert_eq!(STATE_INIT.derive_account_id(), deserialized.derive_account_id());
    }

    #[test]
    fn state_init_json_roundtrip() {
        let deserialized = assert_json_roundtrip(&*STATE_INIT);

        assert_eq!(STATE_INIT.derive_account_id(), deserialized.derive_account_id());
    }

    #[test]
    fn lazy_state_init_borsh_roundtrip() {
        let state_init = STATE_INIT.lazy_serialized();
        let deserialized = assert_borsh_roundtrip(&state_init);

        assert_eq!(state_init.derive_account_id(), deserialized.derive_account_id());
    }

    #[test]
    fn lazy_state_init_json_roundtrip() {
        let state_init = STATE_INIT.lazy_serialized();
        let deserialized = assert_json_roundtrip(&state_init);

        assert_eq!(state_init.derive_account_id(), deserialized.derive_account_id());
    }

    #[track_caller]
    fn assert_borsh_roundtrip<T>(value: &T) -> T
    where
        T: BorshSerialize + BorshDeserialize + PartialEq + Debug,
    {
        let serialized = borsh::to_vec(&value).expect("borsh serialize");
        let deserialized: T = borsh::from_slice(&serialized).expect("borsh deserialize");
        assert_eq!(deserialized, *value);
        deserialized
    }

    #[track_caller]
    fn assert_json_roundtrip<T>(value: &T) -> T
    where
        T: Serialize + DeserializeOwned + PartialEq + Debug,
    {
        let serialized = serde_json::to_string_pretty(&value).expect("JSON serialize");
        println!("JSON: {serialized}");
        let deserialized: T = serde_json::from_str(&serialized).expect("JSON deserialize");
        assert_eq!(deserialized, *value);
        deserialized
    }
}
