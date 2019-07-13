use crate::binding::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::{Deref, DerefMut};

pub type AccountId = Vec<u8>;
pub type BlockIndex = u64;
pub type ResultIndex = u32;
pub type Balance = u64;
pub type StorageUsage = u64;

/// Interface that `NearBlockchain` and `MockedBlockchain` implement.
pub trait BlockchainInterface {
    /// Write key/value into the trie.
    fn storage_write<K: Serialize, V: Serialize>(&self, key: &K, value: &V)
    where
        Self: Sized;
    /// Read value based on the key from the trie.
    fn storage_read<K: Serialize, V: DeserializeOwned>(&self, key: &K) -> V
    where
        Self: Sized;
    /// Remove entry from the trie based on the key.
    fn storage_remove<K: Serialize>(&self, key: &K)
    where
        Self: Sized;
    /// Check if trie contains the entry based on the raw key.
    fn storage_has_key<K: Serialize>(&self, key: &K) -> bool
    where
        Self: Sized;

    /// Return the number of results. Only used when the function is called by the callback.
    fn result_count(&self) -> ResultIndex;
    /// Return if result is present.
    fn result_is_ok(&self, index: ResultIndex) -> bool;
    /// Read result by its index.
    fn result_read<R: DeserializeOwned>(&self, index: ResultIndex) -> R
    where
        Self: Sized;

    /// Account that called the initial contract in the chain of promises.
    fn originator_id(&self) -> AccountId;
    /// Account that called this method.
    fn account_id(&self) -> AccountId;

    /// The balance on the account of the smart contract.
    fn frozen_balance(&self) -> Balance;
    /// The balance that can be used for the expenses, like promise and transaction creation.
    fn liquid_balance(&self) -> Balance;
    /// Move balance from liquid to frozen.
    fn deposit(&self, min_amount: Balance, max_amount: Balance) -> Balance;
    /// Move balance from frozen to liquid.
    fn withdraw(&self, min_amount: Balance, max_amount: Balance) -> Balance;
    /// Balance that was attached to the transaction calling the current method.
    fn received_amount(&self) -> Balance;
    /// The current storage usage by the smart contract, including state, code size, account size,
    /// etc.
    fn storage_usage(&self) -> StorageUsage;

    /// Execute assertion.
    fn assert(&self, expr: bool);
    /// Fills given buffer with random u8.
    fn random(&self, buf: &mut [u8]);
    /// Returns the current block index.
    fn block_index(&self) -> BlockIndex;
    /// Records the series of bytes that can be used for debugging the contract.
    fn debug(&self, msg: &[u8]);
}

/// Container that either holds interface to a real blockchain or a mocked blockchain.
pub struct Blockchain {
    bc: Box<dyn BlockchainInterface>,
}

impl Blockchain {
    /// Create a blockchain with mocked interface.
    pub fn injected(mocked_interface: Box<dyn BlockchainInterface>) -> Self {
        Self { bc: mocked_interface }
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self { bc: Box::new(NearBlockchain {}) }
    }
}

impl Deref for Blockchain {
    type Target = dyn BlockchainInterface;
    fn deref(&self) -> &Self::Target {
        self.bc.deref()
    }
}

impl DerefMut for Blockchain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bc.deref_mut()
    }
}

/// Interface of a real Near blockchain.
pub struct NearBlockchain {}

impl BlockchainInterface for NearBlockchain {
    fn storage_write<K: Serialize, V: Serialize>(&self, key: &K, value: &V) {
        let key = bincode::serialize(key).unwrap();
        let value = bincode::serialize(value).unwrap();
        unsafe { storage_write(key.len() as _, key.as_ptr(), value.len() as _, value.as_ptr()) };
    }

    fn storage_read<K: Serialize, V: DeserializeOwned>(&self, key: &K) -> V {
        let key = bincode::serialize(key).unwrap();
        let value = storage_read(key.len() as _, key.as_ptr());
        bincode::deserialize(&value).unwrap()
    }

    fn storage_remove<K: Serialize>(&self, key: &K) {
        let key = bincode::serialize(key).unwrap();
        unsafe { storage_remove(key.len() as _, key.as_ptr()) };
    }

    fn storage_has_key<K: Serialize>(&self, key: &K) -> bool {
        let key = bincode::serialize(key).unwrap();
        unsafe { storage_has_key(key.len() as _, key.as_ptr()) }
    }

    fn result_count(&self) -> ResultIndex {
        unsafe { result_count() }
    }

    fn result_is_ok(&self, index: ResultIndex) -> bool {
        unsafe { result_is_ok(index) }
    }

    fn result_read<R: DeserializeOwned>(&self, index: ResultIndex) -> R {
        let res = result_read(index);
        bincode::deserialize(&res).unwrap()
    }

    fn originator_id(&self) -> AccountId {
        originator_id()
    }

    fn account_id(&self) -> AccountId {
        account_id()
    }

    fn frozen_balance(&self) -> Balance {
        unsafe { frozen_balance() }
    }

    fn liquid_balance(&self) -> Balance {
        unsafe { liquid_balance() }
    }

    fn deposit(&self, min_amount: Balance, max_amount: Balance) -> Balance {
        unsafe { deposit(min_amount, max_amount) }
    }

    fn withdraw(&self, min_amount: Balance, max_amount: Balance) -> Balance {
        unsafe { withdraw(min_amount, max_amount) }
    }

    fn received_amount(&self) -> Balance {
        unsafe { received_amount() }
    }

    fn storage_usage(&self) -> StorageUsage {
        unsafe { storage_usage() }
    }

    fn assert(&self, expr: bool) {
        unsafe { assert(expr) };
    }

    fn random(&self, buf: &mut [u8]) {
        unsafe { random_buf(buf.len() as _, buf.as_mut_ptr()) }
    }

    fn block_index(&self) -> BlockIndex {
        unsafe { block_index() }
    }

    fn debug(&self, msg: &[u8]) {
        log(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::RngCore;
    use std::cell::RefCell;
    use std::cmp::min;
    use std::collections::HashMap;

    /// Mocked version of the blockchain.
    pub struct MockedBlockchain {
        storage: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
        results: RefCell<HashMap<ResultIndex, Vec<u8>>>,
        originator_id: AccountId,
        account_id: AccountId,
        frozen_balance: RefCell<Balance>,
        liquid_balance: RefCell<Balance>,
        received_amount: Balance,
        storage_usage: StorageUsage,
        block_index: BlockIndex,
    }

    impl BlockchainInterface for MockedBlockchain {
        fn storage_write<K: Serialize, V: Serialize>(&self, key: &K, value: &V) {
            let key = bincode::serialize(key).unwrap();
            let value = bincode::serialize(value).unwrap();
            self.storage.borrow_mut().insert(key, value);
        }

        fn storage_read<K: Serialize, V: DeserializeOwned>(&self, key: &K) -> V {
            let key = bincode::serialize(key).unwrap();
            let value = self.storage.borrow().get(&key).unwrap();
            bincode::deserialize(&value).unwrap()
        }

        fn storage_remove<K: Serialize>(&self, key: &K) {
            let key = bincode::serialize(key).unwrap();
            self.storage.borrow_mut().remove(&key);
        }

        fn storage_has_key<K: Serialize>(&self, key: &K) -> bool {
            let key = bincode::serialize(key).unwrap();
            self.storage.borrow().contains_key(&key)
        }

        fn result_count(&self) -> u32 {
            self.results.borrow().len() as _
        }

        fn result_is_ok(&self, index: u32) -> bool {
            self.results.borrow().contains_key(&index)
        }

        fn result_read<R: DeserializeOwned>(&self, index: u32) -> R {
            let result = self.results.borrow().get(&index).unwrap();
            bincode::deserialize(&result).unwrap()
        }

        fn originator_id(&self) -> Vec<u8> {
            self.originator_id.clone()
        }

        fn account_id(&self) -> Vec<u8> {
            self.account_id.clone()
        }

        fn frozen_balance(&self) -> u64 {
            *self.frozen_balance.borrow()
        }

        fn liquid_balance(&self) -> u64 {
            *self.liquid_balance.borrow()
        }

        fn deposit(&self, min_amount: u64, max_amount: u64) -> u64 {
            if *self.liquid_balance.borrow() < min_amount {
                return 0;
            }
            let delta = min(*self.liquid_balance.borrow(), max_amount);
            self.liquid_balance.borrow_mut() -= delta;
            self.frozen_balance.borrow_mut() += delta;
            delta
        }

        fn withdraw(&self, min_amount: u64, max_amount: u64) -> u64 {
            if *self.frozen_balance.borrow() < min_amount {
                return 0;
            }
            let delta = min(*self.frozen_balance.borrow(), max_amount);
            self.frozen_balance.borrow_mut() -= delta;
            self.liquid_balance.borrow_mut() += delta;
            delta
        }

        fn received_amount(&self) -> u64 {
            self.received_amount
        }

        fn storage_usage(&self) -> u64 {
            self.storage_usage
        }

        fn assert(&self, expr: bool) {
            self.assert(expr)
        }

        fn random(&self, buf: &mut [u8]) {
            rand::thread_rng().fill_bytes(buf);
        }

        fn block_index(&self) -> BlockIndex {
            self.block_index
        }

        fn debug(&self, msg: &[u8]) {
            let msg = String::from_utf8(msg.to_vec()).unwrap();
            log::debug!("{}", msg);
        }
    }
}
