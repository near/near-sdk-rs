pub(crate) mod test_env {
    use crate::{env, MockedBlockchain};
    use near_vm_logic::types::AccountId;
    use near_vm_logic::{VMConfig, VMContext};

    /// Objects stored on the trie directly should have identifiers. If identifier is not provided
    /// explicitly than `Default` trait would use this index to generate an id.
    pub(crate) static mut NEXT_TRIE_OBJECT_INDEX: u64 = 0;
    /// Get next id of the object stored on trie.
    pub(crate) fn next_trie_id() -> Vec<u8> {
        unsafe {
            let id = NEXT_TRIE_OBJECT_INDEX;
            NEXT_TRIE_OBJECT_INDEX += 1;
            id.to_le_bytes().to_vec()
        }
    }

    fn alice() -> AccountId {
        "alice.near".to_string()
    }

    fn bob() -> AccountId {
        "bob.near".to_string()
    }

    fn carol() -> AccountId {
        "carol.near".to_string()
    }

    fn setup_with_config(vm_config: VMConfig) {
        let context = VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: carol(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        };
        let storage = match env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };
        env::set_blockchain_interface(Box::new(MockedBlockchain::new(
            context,
            vm_config,
            Default::default(),
            vec![],
            storage,
            Default::default(),
        )));
    }

    pub(crate) fn setup() {
        setup_with_config(VMConfig::default());
    }

    // free == effectively unlimited gas
    pub(crate) fn setup_free() {
        setup_with_config(VMConfig::free());
    }
}
