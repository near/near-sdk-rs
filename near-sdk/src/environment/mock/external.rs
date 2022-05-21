use near_primitives::types::TrieNodesCount;
use near_primitives_core::hash::{hash, CryptoHash};
use near_primitives_core::types::{AccountId, Balance};
use near_vm_logic::{External, ValuePtr};
use std::collections::HashMap;

type Result<T> = ::core::result::Result<T, near_vm_logic::VMLogicError>;

#[derive(Default, Clone)]
/// Emulates the trie and the mock handling code for the SDK. This is a modified version of
/// `MockedExternal` from `near_vm_logic`.
pub(crate) struct SdkExternal {
    pub fake_trie: HashMap<Vec<u8>, Vec<u8>>,
    pub validators: HashMap<AccountId, Balance>,
    data_count: u64,
}

pub struct MockedValuePtr {
    value: Vec<u8>,
}

impl ValuePtr for MockedValuePtr {
    fn len(&self) -> u32 {
        self.value.len() as u32
    }

    fn deref(&self) -> Result<Vec<u8>> {
        Ok(self.value.clone())
    }
}

impl SdkExternal {
    pub fn new() -> Self {
        Self::default()
    }
}

impl External for SdkExternal {
    fn storage_set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.fake_trie.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn storage_get(&self, key: &[u8]) -> Result<Option<Box<dyn ValuePtr>>> {
        Ok(self
            .fake_trie
            .get(key)
            .map(|value| Box::new(MockedValuePtr { value: value.clone() }) as Box<_>))
    }

    fn storage_remove(&mut self, key: &[u8]) -> Result<()> {
        self.fake_trie.remove(key);
        Ok(())
    }

    fn storage_remove_subtree(&mut self, prefix: &[u8]) -> Result<()> {
        self.fake_trie.retain(|key, _| !key.starts_with(prefix));
        Ok(())
    }

    fn storage_has_key(&mut self, key: &[u8]) -> Result<bool> {
        Ok(self.fake_trie.contains_key(key))
    }

    fn generate_data_id(&mut self) -> CryptoHash {
        // Generates some hash for the data ID to receive data. This hash should not be functionally
        // used in any mocked contexts.
        let data_id = hash(&self.data_count.to_le_bytes());
        self.data_count += 1;
        data_id
    }

    fn get_trie_nodes_count(&self) -> TrieNodesCount {
        TrieNodesCount { db_reads: 0, mem_reads: 0 }
    }

    fn validator_stake(&self, account_id: &AccountId) -> Result<Option<Balance>> {
        Ok(self.validators.get(account_id).cloned())
    }

    fn validator_total_stake(&self) -> Result<Balance> {
        Ok(self.validators.values().sum())
    }
}
