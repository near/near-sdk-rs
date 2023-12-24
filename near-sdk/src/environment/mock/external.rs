use near_primitives::types::TrieNodesCount;
use near_primitives_core::hash::{hash, CryptoHash};
use near_primitives_core::types::Balance;
use near_account_id::AccountId;
use near_vm_runner::logic::{External, StorageGetMode, ValuePtr};
use std::collections::HashMap;

type Result<T> = ::core::result::Result<T, near_vm_runner::logic::errors::VMLogicError>;

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

    fn storage_get(
        &self,
        key: &[u8],
        _storage_get_mode: StorageGetMode,
    ) -> Result<Option<Box<dyn ValuePtr>>> {
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

    fn storage_has_key(&mut self, key: &[u8], _mode: StorageGetMode) -> Result<bool> {
        Ok(self.fake_trie.contains_key(key))
    }

    fn generate_data_id(&mut self) -> CryptoHash {
        // Generates some hash for the data ID to receive data. This hash should not be functionally
        // used in any mocked contexts.
        let data_id = hash(&self.data_count.to_le_bytes());
        self.data_count += 1;
        data_id
    }

    fn get_trie_nodes_count(&self) -> near_primitives::types::TrieNodesCount {
        TrieNodesCount { db_reads: 0, mem_reads: 0 }
    }

    fn validator_stake(&self, account_id: &AccountId) -> Result<Option<Balance>> {
        Ok(self.validators.get(account_id).cloned())
    }

    fn validator_total_stake(&self) -> Result<Balance> {
        Ok(self.validators.values().sum())
    }

    fn create_receipt(
        &mut self,
        receipt_indices: Vec<near_vm_runner::logic::types::ReceiptIndex>,
        receiver_id: AccountId,
    ) -> Result<near_vm_runner::logic::types::ReceiptIndex> {
        todo!()
    }

    fn append_action_create_account(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
    ) -> Result<()> {
        todo!()
    }

    fn append_action_deploy_contract(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        code: Vec<u8>,
    ) -> Result<()> {
        todo!()
    }

    fn append_action_function_call_weight(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        method_name: Vec<u8>,
        args: Vec<u8>,
        attached_deposit: Balance,
        prepaid_gas: near_primitives_core::types::Gas,
        gas_weight: near_primitives_core::types::GasWeight,
    ) -> Result<()> {
        todo!()
    }

    fn append_action_transfer(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        deposit: Balance,
    ) -> Result<()> {
    todo!()
    }

    fn append_action_stake(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        stake: Balance,
        public_key: near_crypto::PublicKey,
    ) {
        todo!()
    }

    fn append_action_add_key_with_full_access(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        public_key: near_crypto::PublicKey,
        nonce: near_primitives_core::types::Nonce,
    ) {
        todo!()
    }

    fn append_action_add_key_with_function_call(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        public_key: near_crypto::PublicKey,
        nonce: near_primitives_core::types::Nonce,
        allowance: Option<Balance>,
        receiver_id: AccountId,
        method_names: Vec<Vec<u8>>,
    ) -> Result<()> {
        todo!()
    }

    fn append_action_delete_key(&mut self, receipt_index: near_vm_runner::logic::types::ReceiptIndex, public_key: near_crypto::PublicKey) {
        todo!()
    }

    fn append_action_delete_account(
        &mut self,
        receipt_index: near_vm_runner::logic::types::ReceiptIndex,
        beneficiary_id: AccountId,
    ) -> Result<()> {
        todo!()
    }

    fn get_receipt_receiver(&self, receipt_index: near_vm_runner::logic::types::ReceiptIndex) -> &AccountId {
        todo!()
    }
}
