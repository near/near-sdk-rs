use crate::test_utils::test_env::*;
use crate::{AccountId, Balance, BlockHeight, EpochHeight, PublicKey, VMContext};

/// Simple VMContext builder that allows to quickly create custom context in tests.
pub struct VMContextBuilder {
    pub context: VMContext,
}

#[allow(dead_code)]
impl VMContextBuilder {
    pub fn new() -> Self {
        Self {
            context: VMContext {
                current_account_id: alice(),
                signer_account_id: bob(),
                signer_account_pk: vec![0u8; 32],
                predecessor_account_id: bob(),
                input: vec![],
                block_index: 0,
                block_timestamp: 0,
                epoch_height: 0,
                account_balance: 10u128.pow(26),
                account_locked_balance: 0,
                storage_usage: 1024 * 300,
                attached_deposit: 0,
                prepaid_gas: 300 * 10u64.pow(12),
                random_seed: vec![0u8; 32],
                is_view: false,
                output_data_receivers: vec![],
            },
        }
    }

    pub fn current_account_id(&mut self, account_id: AccountId) -> &mut Self {
        self.context.current_account_id = account_id;
        self
    }

    pub fn signer_account_id(&mut self, account_id: AccountId) -> &mut Self {
        self.context.signer_account_id = account_id;
        self
    }

    pub fn signer_account_pk(&mut self, pk: PublicKey) -> &mut Self {
        self.context.signer_account_pk = pk;
        self
    }

    pub fn predecessor_account_id(&mut self, account_id: AccountId) -> &mut Self {
        self.context.predecessor_account_id = account_id;
        self
    }

    pub fn block_index(&mut self, block_index: BlockHeight) -> &mut Self {
        self.context.block_index = block_index;
        self
    }

    pub fn epoch_height(&mut self, epoch_height: EpochHeight) -> &mut Self {
        self.context.epoch_height = epoch_height;
        self
    }

    pub fn attached_deposit(&mut self, amount: Balance) -> &mut Self {
        self.context.attached_deposit = amount;
        self
    }

    pub fn account_balance(&mut self, amount: Balance) -> &mut Self {
        self.context.account_balance = amount;
        self
    }

    pub fn account_locked_balance(&mut self, amount: Balance) -> &mut Self {
        self.context.account_locked_balance = amount;
        self
    }

    pub fn is_view(&mut self, is_view: bool) -> &mut Self {
        self.context.is_view = is_view;
        self
    }

    pub fn build(&self) -> VMContext {
        self.context.clone()
    }
}
