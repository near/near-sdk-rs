use near_sdk::{AccountId, Balance, BlockHeight, MockedBlockchain, PromiseResult, PublicKey, VMContext};

/// Simple VMContext builder that allows to quickly create custom context in tests.
pub struct VMContextBuilder {
    context: VMContext,
}

impl VMContextBuilder {
    pub fn new() -> Self {
        Self {
            context: VMContext {
                current_account_id: "".to_string(),
                signer_account_id: "".to_string(),
                signer_account_pk: vec![0u8; 32],
                predecessor_account_id: "".to_string(),
                input: vec![],
                block_index: 0,
                block_timestamp: 0,
                epoch_height: 0,
                account_balance: 10u128.pow(26),
                account_locked_balance: 0,
                storage_usage: 1024 * 300,
                attached_deposit: 0,
                prepaid_gas: 10u64.pow(18),
                random_seed: vec![0u8; 32],
                is_view: false,
                output_data_receivers: vec![],
            },
        }
    }

    pub fn current_account_id(mut self, account_id: AccountId) -> Self {
        self.context.current_account_id = account_id;
        self
    }

    pub fn signer_account_id(mut self, account_id: AccountId) -> Self {
        self.context.signer_account_id = account_id;
        self
    }

    pub fn signer_account_pk(mut self, pk: PublicKey) -> Self {
        self.context.signer_account_pk = pk;
        self
    }

    pub fn predecessor_account_id(mut self, account_id: AccountId) -> Self {
        self.context.predecessor_account_id = account_id;
        self
    }

    pub fn block_index(mut self, block_index: BlockHeight) -> Self {
        self.context.block_index = block_index;
        self
    }

    pub fn block_timestamp(mut self, block_timestamp: u64) -> Self {
        self.context.block_timestamp = block_timestamp;
        self
    }

    pub fn attached_deposit(mut self, amount: Balance) -> Self {
        self.context.attached_deposit = amount;
        self
    }

    pub fn account_balance(mut self, amount: Balance) -> Self {
        self.context.account_balance = amount;
        self
    }

    pub fn account_locked_balance(mut self, amount: Balance) -> Self {
        self.context.account_locked_balance = amount;
        self
    }

    pub fn finish(self) -> VMContext {
        self.context
    }
}

pub fn testing_env_with_promise_results(context: VMContext, promise_result: PromiseResult) {
    let storage = near_sdk::env::take_blockchain_interface()
        .unwrap()
        .as_mut_mocked_blockchain()
        .unwrap()
        .take_storage();

    near_sdk::env::set_blockchain_interface(Box::new(MockedBlockchain::new(
        context,
        Default::default(),
        Default::default(),
        vec![promise_result],
        storage,
        Default::default(),
    )));
}

pub fn accounts(id: usize) -> AccountId {
    ["alice", "bob", "charlie", "danny", "eugene", "fargo"][id].to_string()
}