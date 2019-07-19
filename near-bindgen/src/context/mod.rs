//! Context allows smart contract to access the blockchain interface.
use lazy_static::lazy_static;

pub mod option_box;

#[cfg(feature = "env_test")]
pub mod mocked_context;

lazy_static! {
    pub static ref CONTEXT: option_box::BoxOption<dyn Context> = option_box::BoxOption::new();
}

pub type AccountId = Vec<u8>;
pub type BlockIndex = u64;
pub type ResultIndex = u32;
pub type Balance = u64;
pub type StorageUsage = u64;
pub type IteratorId = u32;
pub type PromiseIndex = u32;
pub type DataTypeIndex = u32;

pub const DATA_TYPE_ORIGINATOR_ACCOUNT_ID: DataTypeIndex = 1;
pub const DATA_TYPE_CURRENT_ACCOUNT_ID: DataTypeIndex = 2;
pub const DATA_TYPE_STORAGE: DataTypeIndex = 3;
pub const DATA_TYPE_INPUT: DataTypeIndex = 4;
pub const DATA_TYPE_RESULT: DataTypeIndex = 5;
pub const DATA_TYPE_STORAGE_ITER: DataTypeIndex = 6;

/// Scratch buffer.
const MAX_BUF_SIZE: usize = 1 << 16;
static mut SCRATCH_BUF: Vec<u8> = Vec::new();

/// Interface that `NearContext` and `MockedContext` implement.
pub trait Context {
    /// Write key/value into the trie.
    fn storage_write(&self, key: &[u8], value: &[u8]);
    /// Create iterator that iterates over records with the given prefix. Returns id of the iterator.
    fn storage_iter(&self, prefix: &[u8]) -> IteratorId;
    /// Create iterator that iterates over the records with the keys between `start` and `end`,
    /// excluding `end`. Returns id of the iterator.
    fn storage_range(&self, start: &[u8], end: &[u8]) -> IteratorId;
    /// Advanced iterator, given iterator id. Returns `true` if iterator is not finished yet.
    fn storage_iter_next(&self, iter_id: IteratorId) -> bool;
    /// Remove entry from the trie based on the key.
    fn storage_remove(&self, key: &[u8]);
    /// Check if trie contains the entry based on the key.
    fn storage_has_key(&self, key: &[u8]) -> bool;

    /// Return the number of results. Only used when the function is called by the callback.
    fn result_count(&self) -> ResultIndex;
    /// Return if result is present.
    fn result_is_ok(&self, index: ResultIndex) -> bool;
    /// Records the given value as the execution result of the current contract method.
    fn return_value(&self, value: &[u8]);
    /// Records the given promise as the execution result of the current contract method.
    fn return_promise(&self, promise_index: PromiseIndex);

    /// A low-level function to read generic data from the Trie owned by the contract. Use
    /// specialized `*_read` wrappers, instead.
    fn data_read(
        &self,
        data_type_index: DataTypeIndex,
        key_len: usize,
        key: u32,
        max_buf_len: usize,
        buf_ptr: *mut u8,
    ) -> usize;

    /// Create a promise that calls the given method of the given contract. The promises are lazy
    /// and asynchronous, meaning they are not executed immediately in a blocking fashion and they
    /// also need to be called with `return_promise`.
    fn promise_create(
        &self,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: Balance,
    ) -> PromiseIndex;
    /// Adds callback to the given promise. `method_name` is the method name of the current contract
    /// that will be called after the original promise finishes.
    fn promise_then(
        &self,
        promise_index: PromiseIndex,
        method_name: &[u8],
        arguments: &[u8],
        amount: Balance,
    ) -> PromiseIndex;
    /// Combines two promises into one that waits for both of them to finish.
    fn promise_and(
        &self,
        promise_index1: PromiseIndex,
        promise_index2: PromiseIndex,
    ) -> PromiseIndex;

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
    fn random_buf(&self, buf: &mut [u8]);
    /// Returns random u32 number.
    fn random32(&self) -> u32;
    /// Returns the current block index.
    fn block_index(&self) -> BlockIndex;
    /// Records the series of bytes that can be used for debugging the contract.
    fn debug(&self, msg: &[u8]);

    /// A wrapper on `data_read` that reads the data into the scratch buffer.
    fn data_read_buffered(
        &self,
        data_type_index: DataTypeIndex,
        key_len: usize,
        key: u32,
    ) -> Vec<u8> {
        unsafe {
            if SCRATCH_BUF.len() == 0 {
                SCRATCH_BUF.resize(MAX_BUF_SIZE, 0);
            }
            let len = self.data_read(
                data_type_index,
                key_len,
                key,
                MAX_BUF_SIZE,
                SCRATCH_BUF.as_mut_ptr(),
            );
            self.assert(len <= MAX_BUF_SIZE);
            SCRATCH_BUF[..len as usize].to_vec()
        }
    }
    /// Reads data from general purpose storage, oppose to the one we use for the metadata.
    fn storage_read(&self, key: &[u8]) -> Vec<u8> {
        self.data_read_buffered(DATA_TYPE_STORAGE, key.len() as _, key.as_ptr() as _)
    }
    /// Returns the value of the record that the iterator is currently pointing at.
    fn storage_peek(&self, iter_id: IteratorId) -> Vec<u8> {
        self.data_read_buffered(DATA_TYPE_STORAGE_ITER, 0, iter_id)
    }
    /// Read the input given to the contract method.
    fn input(&self) -> Vec<u8> {
        self.data_read_buffered(DATA_TYPE_INPUT, 0, 0)
    }
    /// A different name for the debug method.
    fn log(&self, msg: &[u8]) {
        self.debug(msg)
    }
    /// If the current method is called through the callback then we can read the result of the
    /// method that calls this callback. If there are multiple methods that were joined through
    /// combinators then we can read results of any of them using index.
    fn result(&self, index: ResultIndex) -> Vec<u8> {
        self.data_read_buffered(DATA_TYPE_RESULT, 0, index)
    }
    /// Get id of the originator -- the account that called this smart contract method. If this
    /// method call is the result of the callback or promise then it returns the account of the
    /// method that called the callback or created the promise.
    fn originator_id(&self) -> AccountId {
        self.data_read_buffered(DATA_TYPE_ORIGINATOR_ACCOUNT_ID, 0, 0)
    }
    /// Get id of the account that created the very first transaction in the chain of cross-contract
    /// calls.
    fn account_id(&self) -> AccountId {
        self.data_read_buffered(DATA_TYPE_CURRENT_ACCOUNT_ID, 0, 0)
    }
}
