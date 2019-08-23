use crate::environment::blockchain_interface::BlockchainInterface;
use near_vm_logic::types::{
    AccountId, Balance, BlockIndex, Gas, IteratorIndex, PromiseIndex, PromiseResult, PublicKey,
    StorageUsage,
};
use std::mem::size_of;

/// The methods that are available by the smart contracts to call.
/// All methods below panic if their invocation causes smart contract to exceed guest memory or
/// internal limits of the host (like number of registers).
/// This is a safe wrapper around low-level `BlockchainInterface`.
pub struct Environment<T> {
    blockchain_interface: T,
}

const REGISTER_EXPECTED_ERR: &str =
    "Register was expected to have data because we just wrote it into it.";
const RETURN_CODE_ERR: &str = "Unexpected return code.";

/// Register used internally for atomic operations. This register is safe to use by the user,
/// since it only needs to be untouched while methods of `Environment` execute, which is guaranteed
/// guest code is not parallel.
const ATOMIC_OP_REGISTER: u64 = 0;
/// Register used to record evicted values from the storage.
const EVICTED_REGISTER: u64 = std::u64::MAX - 1;
/// Register used to read keys.
const KEY_REGISTER: u64 = std::u64::MAX - 2;
/// Register used to read values.
const VALUE_REGISTER: u64 = std::u64::MAX - 3;

/// A simple macro helper to read blob value coming from host's method.
macro_rules! try_method_into_register {
    ($self:ident, $method:ident ) => {{
        $self.blockchain_interface.$method(ATOMIC_OP_REGISTER);
        $self.read_register(ATOMIC_OP_REGISTER)
    }};
}

/// Same as `try_method_into_register` but expects the data.
macro_rules! method_into_register {
    ($self:ident, $method:ident ) => {{
        try_method_into_register!($self, $method).expect(REGISTER_EXPECTED_ERR)
    }};
}

impl<T: BlockchainInterface> Environment<T> {
    pub fn new(blockchain_interface: T) -> Self {
        Self { blockchain_interface }
    }

    /// Reads the content of the `register_id`. If register is not used returns `None`.
    pub fn read_register(&mut self, register_id: u64) -> Option<Vec<u8>> {
        let len = self.register_len(register_id)?;
        let res = vec![0u8; len as usize];
        self.blockchain_interface.read_register(register_id, res.as_ptr() as _);
        Some(res)
    }
    /// Returns the size of the register. If register is not used returns `None`.
    pub fn register_len(&mut self, register_id: u64) -> Option<u64> {
        let len = self.blockchain_interface.register_len(register_id);
        if len == std::u64::MAX {
            None
        } else {
            Some(len)
        }
    }

    // ###############
    // # Context API #
    // ###############
    /// The id of the account that owns the current contract.
    pub fn current_account_id(&mut self) -> AccountId {
        String::from_utf8(method_into_register!(self, current_account_id)).unwrap()
    }
    /// The id of the account that either signed the original transaction or issued the initial
    /// cross-contract call.
    pub fn signer_account_id(&mut self) -> AccountId {
        String::from_utf8(method_into_register!(self, signer_account_id)).unwrap()
    }

    /// The public key of the account that did the signing.
    pub fn signer_account_pk(&mut self) -> PublicKey {
        method_into_register!(self, signer_account_pk)
    }
    /// The id of the account that was the previous contract in the chain of cross-contract calls.
    /// If this is the first contract, it is equal to `signer_account_id`.
    pub fn predecessor_account_id(&mut self) -> String {
        String::from_utf8(method_into_register!(self, predecessor_account_id)).unwrap()
    }
    /// The input to the contract call serialized as bytes. If input is not provided returns `None`.
    pub fn input(&mut self) -> Option<Vec<u8>> {
        try_method_into_register!(self, input)
    }
    /// Current block index.
    pub fn block_index(&mut self) -> BlockIndex {
        self.blockchain_interface.block_index()
    }
    /// Current total storage usage of this smart contract that this account would be paying for.
    pub fn storage_usage(&mut self) -> StorageUsage {
        self.blockchain_interface.storage_usage()
    }

    // #################
    // # Economics API #
    // #################
    /// The balance attached to the given account. This includes the attached_deposit that was
    /// attached to the transaction
    pub fn account_balance(&mut self) -> Balance {
        let data = [0u8; size_of::<Balance>()];
        self.blockchain_interface.account_balance(data.as_ptr() as u64);
        Balance::from_le_bytes(data)
    }
    /// The balance that was attached to the call that will be immediately deposited before the
    /// contract execution starts
    pub fn attached_deposit(&mut self) -> Balance {
        let data = [0u8; size_of::<Balance>()];
        self.blockchain_interface.attached_deposit(data.as_ptr() as u64);
        Balance::from_le_bytes(data)
    }
    /// The amount of gas attached to the call that can be used to pay for the gas fees.
    pub fn prepaid_gas(&mut self) -> Gas {
        self.blockchain_interface.prepaid_gas()
    }
    /// The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
    pub fn used_gas(&mut self) -> Gas {
        self.blockchain_interface.used_gas()
    }

    // ############
    // # Math API #
    // ############
    /// Get random seed from the register.
    pub fn random_seed(&mut self) -> Vec<u8> {
        method_into_register!(self, random_seed)
    }
    /// Hashes the random sequence of bytes using sha256.
    pub fn sha256(&mut self, value: &[u8]) -> Vec<u8> {
        self.blockchain_interface.sha256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
        self.read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)
    }

    // ################
    // # Promises API #
    // ################
    /// Creates a promise that will execute a method on account with given arguments and attaches
    /// the given amount and gas.
    pub fn promise_create(
        &mut self,
        account_id: AccountId,
        method_name: &[u8],
        arguments: &[u8],
        amount: Balance,
        gas: Gas,
    ) -> PromiseIndex {
        let account_id = account_id.as_bytes();
        self.blockchain_interface.promise_create(
            account_id.len() as _,
            account_id.as_ptr() as _,
            method_name.len() as _,
            method_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount as *const Balance as _,
            gas,
        )
    }
    /// Attaches the callback that is executed after promise pointed by `promise_idx` is complete.
    pub fn promise_then(
        &mut self,
        promise_idx: PromiseIndex,
        account_id: AccountId,
        method_name: &[u8],
        arguments: &[u8],
        amount: Balance,
        gas: Gas,
    ) -> PromiseIndex {
        let account_id = account_id.as_bytes();
        self.blockchain_interface.promise_then(
            promise_idx,
            account_id.len() as _,
            account_id.as_ptr() as _,
            method_name.len() as _,
            method_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount as *const Balance as _,
            gas,
        )
    }
    /// Creates a new promise which completes when time all promises passed as arguments complete.
    pub fn promise_and(&mut self, promise_indices: &[PromiseIndex]) -> PromiseIndex {
        let mut data = vec![0u8; promise_indices.len() * size_of::<PromiseIndex>()];
        for i in 0..promise_indices.len() {
            data[i * size_of::<PromiseIndex>()..(i + 1) * size_of::<PromiseIndex>()]
                .copy_from_slice(&promise_indices[i].to_le_bytes());
        }
        self.blockchain_interface.promise_and(data.as_ptr() as _, promise_indices.len() as _)
    }
    /// If the current function is invoked by a callback we can access the execution results of the
    /// promises that caused the callback. This function returns the number of complete and
    /// incomplete callbacks.
    pub fn promise_results_count(&mut self) -> u64 {
        self.blockchain_interface.promise_results_count()
    }
    /// If the current function is invoked by a callback we can access the execution results of the
    /// promises that caused the callback.
    pub fn promise_result(&mut self, result_idx: u64) -> PromiseResult {
        match self.blockchain_interface.promise_result(result_idx, ATOMIC_OP_REGISTER) {
            0 => PromiseResult::NotReady,
            1 => {
                let data = self
                    .read_register(ATOMIC_OP_REGISTER)
                    .expect("Promise result should've returned into register.");
                PromiseResult::Successful(data)
            }
            2 => PromiseResult::Failed,
            _ => panic!(RETURN_CODE_ERR),
        }
    }
    /// Consider the execution result of promise under `promise_idx` as execution result of this
    /// function.
    pub fn promise_return(&mut self, promise_idx: PromiseIndex) {
        self.blockchain_interface.promise_return(promise_idx)
    }

    // #####################
    // # Miscellaneous API #
    // #####################
    /// Sets the blob of data as the return value of the contract.
    pub fn value_return(&mut self, value: &[u8]) {
        self.blockchain_interface.value_return(value.len() as _, value.as_ptr() as _)
    }
    /// Terminates the execution of the program.
    pub fn panic(&mut self) {
        self.blockchain_interface.panic()
    }
    /// Log the UTF-8 encodable message.
    pub fn log(&mut self, message: &[u8]) {
        self.blockchain_interface.log_utf8(message.len() as _, message.as_ptr() as _)
    }

    // ###############
    // # Storage API #
    // ###############
    /// Writes key-value into storage.
    /// If another key-value existed in the storage with the same key it returns `true`, otherwise `false`.
    pub fn storage_write(&mut self, key: &[u8], value: &[u8]) -> bool {
        match self.blockchain_interface.storage_write(
            key.len() as _,
            key.as_ptr() as _,
            value.len() as _,
            value.as_ptr() as _,
            EVICTED_REGISTER,
        ) {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }
    /// Reads the value stored under the given key.
    pub fn storage_read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        match self.blockchain_interface.storage_read(
            key.len() as _,
            key.as_ptr() as _,
            ATOMIC_OP_REGISTER,
        ) {
            0 => None,
            1 => Some(self.read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)),
            _ => panic!(RETURN_CODE_ERR),
        }
    }
    /// Removes the value stored under the given key.
    /// If key-value existed returns `true`, otherwise `false`.
    pub fn storage_remove(&mut self, key: &[u8]) -> bool {
        match self.blockchain_interface.storage_remove(
            key.len() as _,
            key.as_ptr() as _,
            EVICTED_REGISTER,
        ) {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }
    /// Reads the most recent value that was evicted with `storage_write` or `storage_remove` command.
    pub fn storage_get_evicted(&mut self) -> Option<Vec<u8>> {
        self.read_register(EVICTED_REGISTER)
    }
    /// Checks if there is a key-value in the storage.
    pub fn storage_has_key(&mut self, key: &[u8]) -> bool {
        match self.blockchain_interface.storage_has_key(key.len() as _, key.as_ptr() as _) {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }
    /// Creates an iterator that iterates key-values based on the prefix of the key.
    pub fn storage_iter_prefix(&mut self, prefix: &[u8]) -> IteratorIndex {
        self.blockchain_interface.storage_iter_prefix(prefix.len() as _, prefix.as_ptr() as _)
    }
    /// Creates an iterator that iterates key-values in [start, end) interval.
    pub fn storage_iter_range(&mut self, start: &[u8], end: &[u8]) -> IteratorIndex {
        self.blockchain_interface.storage_iter_range(
            start.len() as _,
            start.as_ptr() as _,
            end.len() as _,
            end.as_ptr() as _,
        )
    }
    /// Checks the next element of iterator progressing it. Returns `true` if the element is available.
    /// Returns `false` if iterator has finished.
    pub fn storage_iter_next(&mut self, iterator_idx: IteratorIndex) -> bool {
        match self.blockchain_interface.storage_iter_next(
            iterator_idx,
            KEY_REGISTER,
            VALUE_REGISTER,
        ) {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }
    /// Reads the key that iterator was pointing to.
    pub fn storage_iter_key_read(&mut self) -> Option<Vec<u8>> {
        self.read_register(KEY_REGISTER)
    }
    /// Reads the value that iterator was pointing to.
    pub fn storage_iter_value_read(&mut self) -> Option<Vec<u8>> {
        self.read_register(VALUE_REGISTER)
    }
}
