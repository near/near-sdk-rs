use near_vm_logic::types::{Balance, StorageUsage, BlockIndex, AccountId, PublicKey, Gas, PromiseIndex, PromiseResult, IteratorIndex};

/// The methods that are available by the smart contracts to call.
/// All methods below panic if their invocation causes smart contract to exceed guest memory or
/// internal limits of the host (like number of registers).
trait Environment {
    /// Reads the content of the `register_id`.
    ///
    /// # Panics:
    /// * If `register_id` is pointing to an unused register.
    fn read_register(&self, register_id: u64) -> Vec<u8>;

    /// Returns the size of the register. If register is not used returns `None`.
    fn register_len(&self, register_id: u64) -> Option<u64>;

    // ###############
    // # Context API #
    // ###############
    /// The id of the account that owns the current contract.
    fn current_account_id(&self) -> AccountId;
    /// The id of the account that either signed the original transaction or issued the initial
    /// cross-contract call.
    fn signer_account_id(&self) -> AccountId;
    /// The public key of the account that did the signing.
    fn signer_account_pk(&self) -> PublicKey;
    /// The id of the account that was the previous contract in the chain of cross-contract calls.
    /// If this is the first contract, it is equal to `signer_account_id`.
    fn predecessor_account_id(&self) -> AccountId;
    /// The input to the contract call serialized as bytes. If input is not provided returns `None`.
    fn input(&self) -> Option<Vec<u8>>;
    /// Current block index.
    fn block_index(&self) -> BlockIndex;
    /// Current total storage usage of this smart contract that this account would be paying for.
    fn storage_usage(&self) -> StorageUsage;

    // #################
    // # Economics API #
    // #################
    /// The balance attached to the given account. This includes the attached_deposit that was
    /// attached to the transaction
    fn account_balance(&self) -> Balance;
    /// The balance that was attached to the call that will be immediately deposited before the
    /// contract execution starts
    fn attached_deposit(&self) -> Balance;
    /// The amount of gas attached to the call that can be used to pay for the gas fees.
    fn prepaid_gas(&self) -> Gas;
    /// The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
    fn used_gas(&self) -> Gas;

    // ############
    // # Math API #
    // ############
    /// Get random seed from the register.
    fn random_seed(&self) -> Vec<u8>;
    /// Hashes the random sequence of bytes using sha256.
    fn sha256(&self, value: &[u8]) -> Vec<u8>;

    // ################
    // # Promises API #
    // ################
    /// Creates a promise that will execute a method on account with given arguments and attaches
    /// the given amount and gas.
    fn promise_create(&self, account_id: AccountId, method_name: &[u8], arguments: &[u8], amount: Balance, gas: Gas) -> PromiseIndex;
    /// Attaches the callback that is executed after promise pointed by `promise_idx` is complete.
    fn promise_then(&self, promise_idx: PromiseIndex, account_id: AccountId, method_name: &[u8], arguments: &[u8], amount: Balance, gas: Gas) -> PromiseIndex;
    /// Creates a new promise which completes when time all promises passed as arguments complete.
    fn promise_and(&self, promise_indices: &[PromiseIndex]) -> PromiseIndex;
    /// If the current function is invoked by a callback we can access the execution results of the
    /// promises that caused the callback. This function returns the number of complete and
    /// incomplete callbacks.
    fn promise_results_count(&self) -> u64;
    /// If the current function is invoked by a callback we can access the execution results of the
    /// promises that caused the callback.
    fn promise_result(&self, result_idx: u64) -> PromiseResult;
    /// Consider the execution result of promise under `promise_idx` as execution result of this
    /// function.
    fn promise_return(&self, promise_idx: PromiseIndex);

    // #####################
    // # Miscellaneous API #
    // #####################
    /// Sets the blob of data as the return value of the contract.
    fn value_return(&self, value: &[u8]);
    /// Terminates the execution of the program.
    fn panic();
    fn log(&self, message: &[u8]);

    // ###############
    // # Storage API #
    // ###############
    /// Writes key-value into storage.
    /// If another key-value existed in the storage with the same key it returns `true`, otherwise `false`.
    fn storage_write(&mut self, key: &[u8], value: &[u8]) -> bool;
    /// Reads the value stored under the given key.
    fn storage_read(&self, key: &[u8]) -> Option<Vec<u8>>;
    /// Removes the value stored under the given key.
    /// If key-value existed returns `true`, otherwise `false`.
    fn storage_remove(&mut self, key: &[u8]) -> bool;
    /// Reads the most recent value that was evicted with `storage_write` or `storage_remove` command.
    fn storage_get_evicted(&self, key: &[u8]) -> Option<Vec<u8>>;
    /// Checks if there is a key-value in the storage.
    fn storage_has_key(&self, key: &[u8]) -> bool;
    /// Creates an iterator that iterates key-values based on the prefix of the key.
    fn storage_iter_prefix(&self, prefix: &[u8]) -> IteratorIndex;
    /// Creates an iterator that iterates key-values in [start, end) interval.
    fn storage_iter_range(&self, start: &[u8], end: &[u8]) -> IteratorIndex;
    /// Checks the next element of iterator progressing it. Returns `true` if the element is available.
    /// Returns `false` if iterator has finished.
    fn storage_iter_next(&self, iterator_idx: IteratorIndex) -> bool;
    /// Reads the key that iterator was pointing to.
    fn storage_iter_key_read(&self) -> Option<Vec<u8>>;
    /// Reads the value that iterator was pointing to.
    fn storage_iter_value_read(&self) -> Option<Vec<u8>>;
}

