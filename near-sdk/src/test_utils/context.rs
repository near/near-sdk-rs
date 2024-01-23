use crate::mock::MockedBlockchain;
use crate::test_utils::test_env::*;
use crate::{test_vm_config, AccountId};
use crate::{BlockHeight, EpochHeight, Gas, NearToken, PromiseResult, PublicKey, StorageUsage};
use near_parameters::RuntimeFeesConfig;
use near_primitives_core::config::ViewConfig;
use std::convert::TryInto;

/// Returns a pre-defined account_id from a list of 6.
pub fn accounts(id: usize) -> AccountId {
    ["alice", "bob", "charlie", "danny", "eugene", "fargo"][id].parse().unwrap()
}

/// Simple VMContext builder that allows to quickly create custom context in tests.
#[derive(Clone)]
pub struct VMContextBuilder {
    pub context: VMContext,
}

impl Default for VMContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
/// Context for the contract execution.
pub struct VMContext {
    /// The account id of the current contract that we are executing.
    pub current_account_id: AccountId,
    /// The account id of that signed the original transaction that led to this
    /// execution.
    pub signer_account_id: AccountId,
    /// The public key that was used to sign the original transaction that led to
    /// this execution.
    pub signer_account_pk: PublicKey,
    /// If this execution is the result of cross-contract call or a callback then
    /// predecessor is the account that called it.
    /// If this execution is the result of direct execution of transaction then it
    /// is equal to `signer_account_id`.
    pub predecessor_account_id: AccountId,
    /// The input to the contract call.
    /// Encoded as base64 string to be able to pass input in borsh binary format.
    pub input: Vec<u8>,
    /// The current block height.
    pub block_index: BlockHeight,
    /// The current block timestamp (number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC).
    pub block_timestamp: u64,
    /// The current epoch height.
    pub epoch_height: EpochHeight,

    /// The balance attached to the given account. Excludes the `attached_deposit` that was
    /// attached to the transaction.
    pub account_balance: NearToken,
    /// The balance of locked tokens on the given account.
    pub account_locked_balance: NearToken,
    /// The account's storage usage before the contract execution
    pub storage_usage: StorageUsage,
    /// The balance that was attached to the call that will be immediately deposited before the
    /// contract execution starts.
    pub attached_deposit: NearToken,
    /// The gas attached to the call that can be used to pay for the gas fees.
    pub prepaid_gas: Gas,
    /// Initial seed for randomness
    pub random_seed: [u8; 32],
    /// If Some, it means that execution is made in a view mode and defines its configuration.
    /// View mode means that only read-only operations are allowed.
    /// See <https://nomicon.io/Proposals/0018-view-change-method.html> for more details.
    pub view_config: Option<ViewConfig>,
    /// How many `DataReceipt`'s should receive this execution result. This should be empty if
    /// this function call is a part of a batch and it is not the last action.
    pub output_data_receivers: Vec<AccountId>,
}

impl VMContext {
    pub fn is_view(&self) -> bool {
        self.view_config.is_some()
    }
}

#[allow(dead_code)]
impl VMContextBuilder {
    pub fn new() -> Self {
        Self {
            context: VMContext {
                current_account_id: alice(),
                signer_account_id: bob(),
                signer_account_pk: vec![0u8; 33].try_into().unwrap(),
                predecessor_account_id: bob(),
                input: vec![],
                block_index: 0,
                block_timestamp: 0,
                epoch_height: 0,
                account_balance: NearToken::from_yoctonear(10u128.pow(26)),
                account_locked_balance: NearToken::from_near(0),
                storage_usage: 1024 * 300,
                attached_deposit: NearToken::from_near(0),
                prepaid_gas: Gas::from_tgas(300),
                random_seed: [0u8; 32],
                view_config: None,
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

    #[deprecated(since = "4.1.2", note = "Use `block_height` method instead")]
    pub fn block_index(&mut self, block_index: BlockHeight) -> &mut Self {
        self.context.block_index = block_index;
        self
    }

    pub fn block_height(&mut self, block_height: BlockHeight) -> &mut Self {
        self.context.block_index = block_height;
        self
    }

    pub fn block_timestamp(&mut self, block_timestamp: u64) -> &mut Self {
        self.context.block_timestamp = block_timestamp;
        self
    }

    pub fn epoch_height(&mut self, epoch_height: EpochHeight) -> &mut Self {
        self.context.epoch_height = epoch_height;
        self
    }

    pub fn account_balance(&mut self, amount: NearToken) -> &mut Self {
        self.context.account_balance = amount;
        self
    }

    pub fn account_locked_balance(&mut self, amount: NearToken) -> &mut Self {
        self.context.account_locked_balance = amount;
        self
    }

    pub fn storage_usage(&mut self, usage: StorageUsage) -> &mut Self {
        self.context.storage_usage = usage;
        self
    }

    pub fn attached_deposit(&mut self, amount: NearToken) -> &mut Self {
        self.context.attached_deposit = amount;
        self
    }

    pub fn prepaid_gas(&mut self, gas: Gas) -> &mut Self {
        self.context.prepaid_gas = gas;
        self
    }

    pub fn random_seed(&mut self, seed: [u8; 32]) -> &mut Self {
        self.context.random_seed = seed;
        self
    }

    pub fn is_view(&mut self, is_view: bool) -> &mut Self {
        self.context.view_config =
            if is_view { Some(ViewConfig { max_gas_burnt: 200000000000000 }) } else { None };
        self
    }

    pub fn build(&self) -> VMContext {
        self.context.clone()
    }
}

/// Initializes the [`MockedBlockchain`] with a single promise result during execution.
#[deprecated(since = "4.0.0", note = "Use `testing_env!` macro to initialize with promise results")]
pub fn testing_env_with_promise_results(context: VMContext, promise_result: PromiseResult) {
    let storage = crate::mock::with_mocked_blockchain(|b| b.take_storage());

    //? This probably shouldn't need to replace the existing mocked blockchain altogether?
    //? Might be a good time to remove this utility function altogether
    crate::env::set_blockchain_interface(MockedBlockchain::new(
        context,
        test_vm_config(),
        RuntimeFeesConfig::test(),
        vec![promise_result],
        storage,
        Default::default(),
        None,
    ));
}
