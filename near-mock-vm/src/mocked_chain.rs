use serde::Serialize;

use crate::memory::*;
use crate::utils::*;
use crate::runner;
use near_sdk::*;
use near_vm_logic::MemoryLike;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;


// lifted from the `console_log` example
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(a: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
type Storage = BTreeMap<Vec<u8>, Vec<u8>>;


fn new_chain(context: VMContext, storage: Option<Storage>, memory: MockedMemory) -> MockedBlockchain {
    let storage = storage.unwrap_or_default();
    let config = VMConfig::default();
    let fees_config = RuntimeFeesConfig::default();
    let memory_opt: Option<Box<dyn MemoryLike>> = Some(Box::new(memory));

    MockedBlockchain::new(context, config, fees_config, vec![], storage, memory_opt)
}

#[wasm_bindgen]
pub fn inject_contract(wasm_bytes: JsValue) -> JsValue {
    let bytes: Vec<u8> = serde_wasm_bindgen::from_value(wasm_bytes).unwrap();
    let instrumented_bytes = runner::prepare_contract(&bytes, &VMConfig::default()).unwrap();
    serde_wasm_bindgen::to_value(&instrumented_bytes).unwrap()
}

#[wasm_bindgen]
pub fn test_memory(mem: NearMemory) {
    let mut memory = MockedMemory { mem };
    let v: Vec<u8> = vec![42 as u8];
    memory.write_memory(0, &v);

}

#[wasm_bindgen]
pub struct VM {
    chain: MockedBlockchain,
    context: VMContext,
    original: VMContext,
    memory: MockedMemory,
}

#[allow(dead_code)]
fn print_str(s: String) {
    console_log!("{}", s)
}

#[wasm_bindgen]
impl VM {
    #[wasm_bindgen(constructor)]
    pub fn new(context: JsValue, mem: NearMemory) -> Self {
        set_panic_hook();
        let memory: MockedMemory = MockedMemory { mem };
        let _context: VMContext = serde_wasm_bindgen::from_value(context).unwrap();
        let context: VMContext = _context.clone();
        let original: VMContext = context.clone();
        let stor_opt: Option<Storage> = None;
        let chain = new_chain(_context, stor_opt, memory.clone());
        Self { chain, context, original, memory}
    }

    fn take_storage(&mut self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        self.chain.take_storage()
    }

    pub fn reset(&mut self) {
      self.chain = new_chain(self.original.clone(), None, self.memory.clone());
    }

    pub fn set_context(&mut self, context: JsValue) {
        self.context = serde_wasm_bindgen::from_value(context).unwrap();
        self.switch_context();
    }

    fn switch_context(&mut self) {
        self.chain = new_chain(self.context.clone(), Some(self.take_storage()), self.memory.clone());
    }

    pub fn set_current_account_id(&mut self, s: JsValue) {
        self.context.current_account_id = serde_wasm_bindgen::from_value(s).unwrap();
        self.switch_context();
    }
    // Base64 string
    pub fn set_input(&mut self, s: JsValue) {
        let input_str: String = serde_wasm_bindgen::from_value(s).unwrap();
        let res = base64::decode(&input_str).unwrap();
        self.context.input = res;
        self.switch_context();
    }

    pub fn set_signer_account_id(&mut self, s: JsValue) {
        self.context.signer_account_id = serde_wasm_bindgen::from_value(s).unwrap();
        self.switch_context();
    }
    // string
    /// The public key that was used to sign the original transaction that led to
    /// this execution.
    pub fn set_signer_account_pk(&mut self, s: JsValue) {
        self.context.signer_account_pk = serde_wasm_bindgen::from_value(s).unwrap();
        self.switch_context();
    }
    // string base58
    pub fn set_predecessor_account_id(&mut self, s: JsValue) {
        self.context.predecessor_account_id = serde_wasm_bindgen::from_value(s).unwrap();
        self.switch_context();
    }
    // string
    pub fn set_block_index(&mut self, block_height: u64) {
        self.context.block_index = block_height;
        self.switch_context();
    }
    // u128
    pub fn set_block_timestamp(&mut self, stmp: u64) {
        self.context.block_timestamp = stmp;
        self.switch_context();
    }

    pub fn set_account_balance(&mut self, lo: u64, hi: u64) {
        self.context.account_balance = u128_from_u64s(lo, hi) + self.context.attached_deposit; // TODO: serde_wasm_bindgen::from_value(_u128).unwrap()
        self.switch_context()
    }

    pub fn set_account_locked_balance(&mut self, lo: u64, hi: u64) {
        self.context.account_locked_balance = u128_from_u64s(lo, hi); // TODO: serde_wasm_bindgen::from_value(_u128).unwrap();
        self.switch_context();
    }

    pub fn set_storage_usage(&mut self, amt: u64) {
        self.context.storage_usage = amt; //serde_wasm_bindgen::from_value(amt).unwrap();
        self.switch_context();
    }

    pub fn set_attached_deposit(&mut self, lo: u64, hi: u64) {
        self.context.attached_deposit = u128_from_u64s(lo, hi); // TODO: serde_wasm_bindgen::from_value(_u128).unwrap();
        self.switch_context();
    }

    pub fn set_prepaid_gas(&mut self, _u64: u64) {
        self.context.prepaid_gas = _u64;
        self.switch_context();
    }

    pub fn set_random_seed(&mut self, s: JsValue) {
        self.context.random_seed = serde_wasm_bindgen::from_value(s).unwrap();
        self.switch_context();
    }

    pub fn set_is_view(&mut self, b: bool) {
        self.context.is_view = b;
        self.switch_context();
    }

    pub fn set_output_data_receivers(&mut self, arr: JsValue) {
        self.context.output_data_receivers = serde_wasm_bindgen::from_value(arr).unwrap();
        self.switch_context();
    }

    /// #################
    /// # Registers API #
    /// #################

    /// Writes the entire content from the register `register_id` into the memory of the guest starting with `ptr`.
    ///
    /// # Arguments
    ///
    /// * `register_id` -- a register id from where to read the data;
    /// * `ptr` -- location on guest memory where to copy the data.
    ///
    /// # Errors
    ///
    /// * If the content extends outside the memory allocated to the guest. In Wasmer, it returns `MemoryAccessViolation` error message;
    /// * If `register_id` is pointing to unused register returns `InvalidRegisterId` error message.
    ///
    /// # Undefined Behavior
    ///
    /// If the content of register extends outside the preallocated memory on the host side, or the pointer points to a
    /// wrong location this function will overwrite memory that it is not supposed to overwrite causing an undefined behavior.
    ///
    /// # Cost
    ///
    /// `base + read_register_base + read_register_byte * num_bytes + write_memory_base + write_memory_byte * num_bytes`
    pub fn read_register(&mut self, register_id: u64, ptr: u64) {
        // let data = &vec![42];
        //self.chain.wrapped_internal_write_register(register_id, &data);
        unsafe { self.chain.read_register(register_id, ptr) }
        // match res {
        //     Ok(()) => (),
        //     // Err(VMLogicError::HostError(_e)) => console_log!("Host Error"),
        //     Err(e) => panic!(e)
        // }
    }

    // Returns the size of the blob stored in the given register.
    // * If register is used, then returns the size, which can potentially be zero;
    // * If register is not used, returns `u64::MAX`
    //
    // # Arguments
    //
    // * `register_id` -- a register id from where to read the data;
    //
    // # Cost
    //
    // `base`
    pub fn register_len(&mut self, register_id: u64) -> u64 {
        unsafe { self.chain.register_len(register_id) }
    }

    // Copies `data` from the guest memory into the register. If register is unused will initialize
    // it. If register has larger capacity than needed for `data` will not re-allocate it. The
    // register will lose the pre-existing data if any.
    //
    // # Arguments
    //
    // * `register_id` -- a register id where to write the data;
    // * `data_len` -- length of the data in bytes;
    // * `data_ptr` -- pointer in the guest memory where to read the data from.
    //
    // # Cost
    //
    // `base + read_memory_base + read_memory_bytes * num_bytes + write_register_base + write_register_bytes * num_bytes`
    // pub fn write_register(&mut self, register_id: u64, data_len: u64, data_ptr: u64) -> () {
    //    self.chain.write_register(register_id, data_len, data_ptr))
    // }
    // TODO
    /// ###################################
    /// # String reading helper functions #
    /// ###################################

    /// Helper function to read and return utf8-encoding string.
    /// If `len == u64::MAX` then treats the string as null-terminated with character `'\0'`.
    ///
    /// # Errors
    ///
    /// * If string extends outside the memory of the guest with `MemoryAccessViolation`;
    /// * If string is not UTF-8 returns `BadUtf8`.
    /// * If string is longer than `max_log_len` returns `BadUtf8`.
    ///
    /// # Cost
    ///
    /// For not nul-terminated string:
    /// `read_memory_base + read_memory_byte * num_bytes + utf8_decoding_base + utf8_decoding_byte * num_bytes`
    ///
    /// For nul-terminated string:
    /// `(read_memory_base + read_memory_byte) * num_bytes + utf8_decoding_base + utf8_decoding_byte * num_bytes`

    /// Helper function to read UTF-16 formatted string from guest memory.
    /// # Errors
    ///
    /// * If string extends outside the memory of the guest with `MemoryAccessViolation`;
    /// * If string is not UTF-16 returns `BadUtf16`.
    ///
    /// # Cost
    ///
    /// For not nul-terminated string:
    /// `read_memory_base + read_memory_byte * num_bytes + utf16_decoding_base + utf16_decoding_byte * num_bytes`
    ///
    /// For nul-terminated string:
    /// `read_memory_base * num_bytes / 2 + read_memory_byte * num_bytes + utf16_decoding_base + utf16_decoding_byte * num_bytes`

    /// ###############
    /// # Context API #
    /// ###############

    /// Saves the account id of the current contract that we execute into the register.
    ///
    /// # Errors
    ///
    /// If the registers exceed the memory limit returns `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes`
    pub fn current_account_id(&mut self, register_id: u64) -> () {
        unsafe { self.chain.current_account_id(register_id) }
    }
    /// All contract calls are a result of some transaction that was signed by some account using
    /// some access key and submitted into a memory pool (either through the wallet using RPC or by
    /// a node itself). This function returns the id of that account. Saves the bytes of the signer
    /// account id into the register.
    ///
    /// # Errors
    ///
    /// * If the registers exceed the memory limit returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes`
    pub fn signer_account_id(&mut self, register_id: u64) -> () {
        unsafe { self.chain.signer_account_id(register_id) }
    }
    /// Saves the public key fo the access key that was used by the signer into the register. In
    /// rare situations smart contract might want to know the exact access key that was used to send
    /// the original transaction, e.g. to increase the allowance or manipulate with the public key.
    ///
    /// # Errors
    ///
    /// * If the registers exceed the memory limit returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes`
    pub fn signer_account_pk(&mut self, register_id: u64) -> () {
        unsafe { self.chain.signer_account_pk(register_id) }
    }
    /// All contract calls are a result of a receipt, this receipt might be created by a transaction
    /// that does function invocation on the contract or another contract as a result of
    /// cross-contract call. Saves the bytes of the predecessor account id into the register.
    ///
    /// # Errors
    ///
    /// * If the registers exceed the memory limit returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes`
    pub fn predecessor_account_id(&mut self, register_id: u64) -> () {
        unsafe { self.chain.predecessor_account_id(register_id) }
    }
    /// Reads input to the contract call into the register. Input is expected to be in JSON-format.
    /// If input is provided saves the bytes (potentially zero) of input into register. If input is
    /// not provided writes 0 bytes into the register.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes`
    pub fn input(&mut self, register_id: u64) -> () {
        unsafe { self.chain.input(register_id) }
    }
    /// Returns the current block height.
    ///
    /// # Cost
    ///
    /// `base`
    /// TODO #1903 rename to `block_height`
    pub fn block_index(&mut self) -> u64 {
        unsafe { self.chain.block_index() }
    }
    /// Returns the current block timestamp.
    ///
    /// # Cost
    ///
    /// `base`
    pub fn block_timestamp(&mut self) -> u64 {
        unsafe { self.chain.block_timestamp() }
    }
    /// Returns the number of bytes used by the contract if it was saved to the trie as of the
    /// invocation. This includes:
    /// * The data written with storage_* functions during current and previous execution;
    /// * The bytes needed to store the access keys of the given account.
    /// * The contract code size
    /// * A small fixed overhead for account metadata.
    ///
    /// # Cost
    ///
    /// `base`
    pub fn storage_usage(&mut self) -> StorageUsage {
        unsafe { self.chain.storage_usage() }
    }
    /// #################
    /// # Economics API #
    /// #################

    /// The current balance of the given account. This includes the attached_deposit that was
    /// attached to the transaction.
    ///
    /// # Cost
    ///
    /// `base + memory_write_base + memory_write_size * 16`
    pub fn account_balance(&mut self, balance_ptr: u64) -> () {
        // self.builder.memory.write_memory(balance_ptr, &self.context.account_balance.to_le_bytes())
        unsafe { self.chain.account_balance(balance_ptr) }
    }
    /// The current amount of tokens locked due to staking.
    ///
    /// # Cost
    ///
    /// `base + memory_write_base + memory_write_size * 16`
    pub fn account_locked_balance(&mut self, balance_ptr: u64) -> () {
        unsafe { self.chain.account_locked_balance(balance_ptr) }
    }
    /// The balance that was attached to the call that will be immediately deposited before the
    /// contract execution starts.
    ///
    /// # Errors
    ///
    /// If called as view function returns `ProhibitedInView``.
    ///
    /// # Cost
    ///
    /// `base + memory_write_base + memory_write_size * 16`
    pub fn attached_deposit(&mut self, balance_ptr: u64) -> () {
        unsafe { self.chain.attached_deposit(balance_ptr) }
    }
    /// The amount of gas attached to the call that can be used to pay for the gas fees.
    ///
    /// # Errors
    ///
    /// If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base`
    pub fn prepaid_gas(&mut self) -> Gas {
        unsafe { self.chain.prepaid_gas() }
    }
    /// The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
    ///
    /// # Errors
    ///
    /// If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base`
    pub fn used_gas(&mut self) -> Gas {
        unsafe { self.chain.used_gas() }
    }
    /// ############
    /// # Math API #
    /// ############

    /// Writes random seed into the register.
    ///
    /// # Errors
    ///
    /// If the size of the registers exceed the set limit `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes`.
    pub fn random_seed(&mut self, register_id: u64) -> () {
        unsafe { self.chain.random_seed(register_id) }
    }
    /// Hashes the random sequence of bytes using sha256 and returns it into `register_id`.
    ///
    /// # Errors
    ///
    /// If `value_len + value_ptr` points outside the memory or the registers use more memory than
    /// the limit with `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes + sha256_base + sha256_byte * num_bytes`
    pub fn sha256(&mut self, value_len: u64, value_ptr: u64, register_id: u64) -> () {
        unsafe { self.chain.sha256(value_len, value_ptr, register_id) }
    }

    /// Hashes the random sequence of bytes using keccak256 and returns it into `register_id`.
    ///
    /// # Errors
    ///
    /// If `value_len + value_ptr` points outside the memory or the registers use more memory than
    /// the limit with `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes + keccak256_base + keccak256_byte * num_bytes`
    pub fn keccak256(&mut self, value_len: u64, value_ptr: u64, register_id: u64) -> () {
        unsafe { self.chain.keccak256(value_len, value_ptr, register_id) }
    }

    /// Hashes the random sequence of bytes using keccak512 and returns it into `register_id`.
    ///
    /// # Errors
    ///
    /// If `value_len + value_ptr` points outside the memory or the registers use more memory than
    /// the limit with `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + write_register_base + write_register_byte * num_bytes + keccak512_base + keccak512_byte * num_bytes`
    pub fn keccak512(&mut self, value_len: u64, value_ptr: u64, register_id: u64) -> () {
        unsafe { self.chain.keccak512(value_len, value_ptr, register_id) }
    }

    /// Called by gas metering injected into Wasm. Counts both towards `burnt_gas` and `used_gas`.
    ///
    /// # Errors
    ///
    /// * If passed gas amount somehow overflows internal gas counters returns `IntegerOverflow`;
    /// * If we exceed usage limit imposed on burnt gas returns `GasLimitExceeded`;
    /// * If we exceed the `prepaid_gas` then returns `GasExceeded`.
    pub fn gas(&mut self, gas_amount: u32) -> () {
        self.chain.gas(gas_amount)
    }
    

    /// ################
    /// # Promises API #
    /// ################

    /// A helper function to pay gas fee for creating a new receipt without actions.
    /// # Args:
    /// * `sir`: whether contract call is addressed to itself;
    /// * `data_dependencies`: other contracts that this execution will be waiting on (or rather
    ///   their data receipts), where bool indicates whether this is sender=receiver communication.
    ///
    /// # Cost
    ///
    /// This is a convenience function that encapsulates several costs:
    /// `burnt_gas := dispatch cost of the receipt + base dispatch cost  cost of the data receipt`
    /// `used_gas := burnt_gas + exec cost of the receipt + base exec cost  cost of the data receipt`
    /// Notice that we prepay all base cost upon the creation of the data dependency, we are going to
    /// pay for the content transmitted through the dependency upon the actual creation of the
    /// DataReceipt.

    /// A helper function to subtract balance on transfer or attached deposit for promises.
    /// # Args:
    /// * `amount`: the amount to deduct from the current account balance.

    /// Creates a promise that will execute a method on account with given arguments and attaches
    /// the given amount and gas. `amount_ptr` point to slices of bytes representing `u128`.
    ///
    /// # Errors
    ///
    /// * If `account_id_len + account_id_ptr` or `method_name_len + method_name_ptr` or
    /// `arguments_len + arguments_ptr` or `amount_ptr + 16` points outside the memory of the guest
    /// or host returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Returns
    ///
    /// Index of the new promise that uniquely identifies it within the current execution of the
    /// method.
    ///
    /// # Cost
    ///
    /// Since `promise_create` is a convenience wrapper around `promise_batch_create` and
    /// `promise_batch_action_function_call`. This also means it charges `base` cost twice.
    pub fn promise_create(
        &mut self,
        account_id_len: u64,
        account_id_ptr: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: Gas,
    ) -> u64 {
        unsafe {
            self.chain.promise_create(
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        }
    }

    /// Attaches the callback that is executed after promise pointed by `promise_idx` is complete.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`;
    /// * If `account_id_len + account_id_ptr` or `method_name_len + method_name_ptr` or
    ///   `arguments_len + arguments_ptr` or `amount_ptr + 16` points outside the memory of the
    ///   guest or host returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Returns
    ///
    /// Index of the new promise that uniquely identifies it within the current execution of the
    /// method.
    ///
    /// # Cost
    ///
    /// Since `promise_create` is a convenience wrapper around `promise_batch_then` and
    /// `promise_batch_action_function_call`. This also means it charges `base` cost twice.
    pub fn promise_then(
        &mut self,
        promise_idx: u64,
        account_id_len: u64,
        account_id_ptr: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) -> u64 {
        unsafe {
            self.chain.promise_then(
                promise_idx,
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        }
    }

    /// Creates a new promise which completes when time all promises passed as arguments complete.
    /// Cannot be used with registers. `promise_idx_ptr` points to an array of `u64` elements, with
    /// `promise_idx_count` denoting the number of elements. The array contains indices of promises
    /// that need to be waited on jointly.
    ///
    /// # Errors
    ///
    /// * If `promise_ids_ptr + 8 * promise_idx_count` extend outside the guest memory returns
    ///   `MemoryAccessViolation`;
    /// * If any of the promises in the array do not correspond to existing promises returns
    ///   `InvalidPromiseIndex`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Returns
    ///
    /// Index of the new promise that uniquely identifies it within the current execution of the
    /// method.
    ///
    /// # Cost
    ///
    /// `base + promise_and_base + promise_and_per_promise * num_promises + cost of reading promise ids from memory`.
    pub fn promise_and(&mut self, promise_idx_ptr: u64, promise_idx_count: u64) -> PromiseIndex {
        unsafe { self.chain.promise_and(promise_idx_ptr, promise_idx_count) }
    }

    /// Creates a new promise towards given `account_id` without any actions attached to it.
    ///
    /// # Errors
    ///
    /// * If `account_id_len + account_id_ptr` points outside the memory of the guest or host
    /// returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Returns
    ///
    /// Index of the new promise that uniquely identifies it within the current execution of the
    /// method.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + cost of reading and decoding the account id + dispatch cost of the receipt`.
    /// `used_gas := burnt_gas + exec cost of the receipt`.
    pub fn promise_batch_create(&mut self, account_id_len: u64, account_id_ptr: u64) -> u64 {
        unsafe { self.chain.promise_batch_create(account_id_len, account_id_ptr) }
    }

    /// Creates a new promise towards given `account_id` without any actions attached, that is
    /// executed after promise pointed by `promise_idx` is complete.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`;
    /// * If `account_id_len + account_id_ptr` points outside the memory of the guest or host
    /// returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Returns
    ///
    /// Index of the new promise that uniquely identifies it within the current execution of the
    /// method.
    ///
    /// # Cost
    ///
    /// `base + cost of reading and decoding the account id + dispatch&execution cost of the receipt
    ///  + dispatch&execution base cost for each data dependency`
    pub fn promise_batch_then(
        &mut self,
        promise_idx: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        unsafe { self.chain.promise_batch_then(promise_idx, account_id_len, account_id_ptr) }
    }

    /// Appends `CreateAccount` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action fee`
    /// `used_gas := burnt_gas + exec action fee`
    pub fn promise_batch_action_create_account(&mut self, promise_idx: u64) -> () {
        unsafe { self.chain.promise_batch_action_create_account(promise_idx) }
    }
    /// Appends `DeployContract` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If `code_len + code_ptr` points outside the memory of the guest or host returns
    /// `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading vector from memory `
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_deploy_contract(
        &mut self,
        promise_idx: u64,
        code_len: u64,
        code_ptr: u64,
    ) -> () {
        unsafe { self.chain.promise_batch_action_deploy_contract(promise_idx, code_len, code_ptr) }
    }

    /// Appends `FunctionCall` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If `method_name_len + method_name_ptr` or `arguments_len + arguments_ptr` or
    /// `amount_ptr + 16` points outside the memory of the guest or host returns
    /// `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading vector from memory
    ///  + cost of reading u128, method_name and arguments from the memory`
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_function_call(
        &mut self,
        promise_idx: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: Gas,
    ) -> () {
        unsafe {
            self.chain.promise_batch_action_function_call(
                promise_idx,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        }
    }

    /// Appends `Transfer` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If `amount_ptr + 16` points outside the memory of the guest or host returns
    /// `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading u128 from memory `
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_transfer(&mut self, promise_idx: u64, amount_ptr: u64) -> () {
        unsafe { self.chain.promise_batch_action_transfer(promise_idx, amount_ptr) }
    }

    /// Appends `Stake` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
    /// * If `amount_ptr + 16` or `public_key_len + public_key_ptr` points outside the memory of the
    /// guest or host returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading public key from memory `
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_stake(
        &mut self,
        promise_idx: u64,
        amount_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) -> () {
        unsafe {
            self.chain.promise_batch_action_stake(
                promise_idx,
                amount_ptr,
                public_key_len,
                public_key_ptr,
            )
        }
    }

    /// Appends `AddKey` action to the batch of actions for the given promise pointed by
    /// `promise_idx`. The access key will have `FullAccess` permission.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
    /// * If `public_key_len + public_key_ptr` points outside the memory of the guest or host
    /// returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading public key from memory `
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_add_key_with_full_access(
        &mut self,
        promise_idx: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
    ) -> () {
        unsafe {
            self.chain.promise_batch_action_add_key_with_full_access(
                promise_idx,
                public_key_len,
                public_key_ptr,
                nonce,
            )
        }
    }

    /// Appends `AddKey` action to the batch of actions for the given promise pointed by
    /// `promise_idx`. The access key will have `FunctionCall` permission.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
    /// * If `public_key_len + public_key_ptr`, `allowance_ptr + 16`,
    /// `receiver_id_len + receiver_id_ptr` or `method_names_len + method_names_ptr` points outside
    /// the memory of the guest or host returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading vector from memory
    ///  + cost of reading u128, method_names and public key from the memory + cost of reading and parsing account name`
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_add_key_with_function_call(
        &mut self,
        promise_idx: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
        allowance_ptr: u64,
        receiver_id_len: u64,
        receiver_id_ptr: u64,
        method_names_len: u64,
        method_names_ptr: u64,
    ) -> () {
        unsafe {
            self.chain.promise_batch_action_add_key_with_function_call(
                promise_idx,
                public_key_len,
                public_key_ptr,
                nonce,
                allowance_ptr,
                receiver_id_len,
                receiver_id_ptr,
                method_names_len,
                method_names_ptr,
            )
        }
    }

    /// Appends `DeleteKey` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If the given public key is not a valid (e.g. wrong length) returns `InvalidPublicKey`.
    /// * If `public_key_len + public_key_ptr` points outside the memory of the guest or host
    /// returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading public key from memory `
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_delete_key(
        &mut self,
        promise_idx: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) -> () {
        unsafe {
            self.chain.promise_batch_action_delete_key(promise_idx, public_key_len, public_key_ptr)
        }
    }

    /// Appends `DeleteAccount` action to the batch of actions for the given promise pointed by
    /// `promise_idx`.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If the promise pointed by the `promise_idx` is an ephemeral promise created by
    /// `promise_and` returns `CannotAppendActionToJointPromise`.
    /// * If `beneficiary_id_len + beneficiary_id_ptr` points outside the memory of the guest or
    /// host returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `burnt_gas := base + dispatch action base fee + dispatch action per byte fee * num bytes + cost of reading and parsing account id from memory `
    /// `used_gas := burnt_gas + exec action base fee + exec action per byte fee * num bytes`
    pub fn promise_batch_action_delete_account(
        &mut self,
        promise_idx: u64,
        beneficiary_id_len: u64,
        beneficiary_id_ptr: u64,
    ) -> () {
        unsafe {
            self.chain.promise_batch_action_delete_account(
                promise_idx,
                beneficiary_id_len,
                beneficiary_id_ptr,
            )
        }
    }

    /// If the current function is invoked by a callback we can access the execution results of the
    /// promises that caused the callback. This function returns the number of complete and
    /// incomplete callbacks.
    ///
    /// Note, we are only going to have incomplete callbacks once we have promise_or combinator.
    ///
    ///
    /// * If there is only one callback returns `1`;
    /// * If there are multiple callbacks (e.g. created through `promise_and`) returns their number;
    /// * If the function was called not through the callback returns `0`.
    ///
    /// # Cost
    ///
    /// `base`
    pub fn promise_results_count(&mut self) -> u64 {
        unsafe { self.chain.promise_results_count() }
    }
    /// If the current function is invoked by a callback we can access the execution results of the
    /// promises that caused the callback. This function returns the result in blob format and
    /// places it into the register.
    ///
    /// * If promise result is complete and successful copies its blob into the register;
    /// * If promise result is complete and failed or incomplete keeps register unused;
    ///
    /// # Returns
    ///
    /// * If promise result is not complete returns `0`;
    /// * If promise result is complete and successful returns `1`;
    /// * If promise result is complete and failed returns `2`.
    ///
    /// # Errors
    ///
    /// * If `result_id` does not correspond to an existing result returns `InvalidPromiseResultIndex`;
    /// * If copying the blob exhausts the memory limit it returns `MemoryAccessViolation`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base + cost of writing data into a register`
    pub fn promise_result(&mut self, result_idx: u64, register_id: u64) -> u64 {
        unsafe { self.chain.promise_result(result_idx, register_id) }
    }
    /// When promise `promise_idx` finishes executing its result is considered to be the result of
    /// the current function.
    ///
    /// # Errors
    ///
    /// * If `promise_idx` does not correspond to an existing promise returns `InvalidPromiseIndex`.
    /// * If called as view function returns `ProhibitedInView`.
    ///
    /// # Cost
    ///
    /// `base + promise_return`
    pub fn promise_return(&mut self, promise_idx: u64) -> () {
        unsafe { self.chain.promise_return(promise_idx) }
    }
    /// #####################
    /// # Miscellaneous API #
    /// #####################

    /// sets the blob of data as the return value of the contract.
    ///
    /// # Errors
    ///
    /// If `value_len + value_ptr` exceeds the memory container or points to an unused register it
    /// returns `MemoryAccessViolation`.
    ///
    /// # Cost
    /// `base + cost of reading return value from memory or register + dispatch&exec cost per byte of the data sent * num data receivers`
    pub fn value_return(&mut self, value_len: u64, value_ptr: u64) -> () {
        unsafe { self.chain.value_return(value_len, value_ptr) }
    }
    /// Terminates the execution of the program with panic `GuestPanic`.
    ///
    /// # Cost
    ///
    /// `base`
    pub fn panic(&mut self) -> () {
        unsafe { self.chain.panic() }
    }
    /// Guest panics with the UTF-8 encoded string.
    /// If `len == u64::MAX` then treats the string as null-terminated with character `'\0'`.
    ///
    /// # Errors
    ///
    /// * If string extends outside the memory of the guest with `MemoryAccessViolation`;
    /// * If string is not UTF-8 returns `BadUtf8`.
    /// * If string is longer than `max_log_len` returns `BadUtf8`.
    ///
    /// # Cost
    /// `base + cost of reading and decoding a utf8 string`
    pub fn panic_utf8(&mut self, len: u64, ptr: u64) -> () {
        unsafe { self.chain.panic_utf8(len, ptr) }
    }
    /// Logs the UTF-8 encoded string.
    /// If `len == u64::MAX` then treats the string as null-terminated with character `'\0'`.
    ///
    /// # Errors
    ///
    /// * If string extends outside the memory of the guest with `MemoryAccessViolation`;
    /// * If string is not UTF-8 returns `BadUtf8`.
    /// * If string is longer than `max_log_len` returns `BadUtf8`.
    ///
    /// # Cost
    ///
    /// `base + log_base + log_byte + num_bytes + utf8 decoding cost`
    pub fn log_utf8(&mut self, len: u64, ptr: u64) -> () {
        unsafe { self.chain.log_utf8(len, ptr) }
    }
    /// Logs the UTF-16 encoded string. If `len == u64::MAX` then treats the string as
    /// null-terminated with two-byte sequence of `0x00 0x00`.
    ///
    /// # Errors
    ///
    /// * If string extends outside the memory of the guest with `MemoryAccessViolation`;
    /// * If string is not UTF-16 returns `BadUtf16`.
    ///
    /// # Cost
    ///
    /// `base + log_base + log_byte * num_bytes + utf16 decoding cost`
    pub fn log_utf16(&mut self, len: u64, ptr: u64) -> () {
        unsafe { self.chain.log_utf16(len, ptr) }
    }
    /// Special import kept for compatibility with AssemblyScript contracts. Not called by smart
    /// contracts directly, but instead called by the code generated by AssemblyScript.
    ///
    /// # Cost
    ///
    /// `base +  log_base + log_byte * num_bytes + utf16 decoding cost`
    pub fn abort(&mut self, msg_ptr: u32, filename_ptr: u32, line: u32, col: u32) -> () {
        unsafe { self.chain.abort(msg_ptr, filename_ptr, line, col) }
    }

    /// ###############
    /// # Storage API #
    /// ###############

    /// Reads account id from the given location in memory.
    ///
    /// # Errors
    ///
    /// * If account is not UTF-8 encoded then returns `BadUtf8`;
    ///
    /// # Cost
    ///
    /// This is a helper function that encapsulates the following costs:
    /// cost of reading buffer from register or memory,
    /// `utf8_decoding_base + utf8_decoding_byte * num_bytes`.

    /// Writes key-value into storage.
    /// * If key is not in use it inserts the key-value pair and does not modify the register. Returns `0`;
    /// * If key is in use it inserts the key-value and copies the old value into the `register_id`. Returns `1`.
    ///
    /// # Errors
    ///
    /// * If `key_len + key_ptr` or `value_len + value_ptr` exceeds the memory container or points
    ///   to an unused register it returns `MemoryAccessViolation`;
    /// * If returning the preempted value into the registers exceed the memory container it returns
    ///   `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + storage_write_base + storage_write_key_byte * num_key_bytes + storage_write_value_byte * num_value_bytes
    /// + get_vec_from_memory_or_register_cost x 2`.
    ///
    /// If a value was evicted it costs additional `storage_write_value_evicted_byte * num_evicted_bytes + internal_write_register_cost`.
    pub fn storage_write(
        &mut self,
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        unsafe {
            // console::log_1(&vm.current_storage_usage.to_string().into());
            let res = self.chain.storage_write(key_len, key_ptr, value_len, value_ptr, register_id);
            // console::log_1(&vm.current_storage_usage.to_string().into());
            res
        }
    }

    /// Reads the value stored under the given key.
    /// * If key is used copies the content of the value into the `register_id`, even if the content
    ///   is zero bytes. Returns `1`;
    /// * If key is not present then does not modify the register. Returns `0`;
    ///
    /// # Errors
    ///
    /// * If `key_len + key_ptr` exceeds the memory container or points to an unused register it
    ///   returns `MemoryAccessViolation`;
    /// * If returning the preempted value into the registers exceed the memory container it returns
    ///   `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + storage_read_base + storage_read_key_byte * num_key_bytes + storage_read_value_byte + num_value_bytes
    ///  cost to read key from register + cost to write value into register`.
    pub fn storage_read(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        unsafe { self.chain.storage_read(key_len, key_ptr, register_id) }
    }
    /// Removes the value stored under the given key.
    /// * If key is used, removes the key-value from the trie and copies the content of the value
    ///   into the `register_id`, even if the content is zero bytes. Returns `1`;
    /// * If key is not present then does not modify the register. Returns `0`.
    ///
    /// # Errors
    ///
    /// * If `key_len + key_ptr` exceeds the memory container or points to an unused register it
    ///   returns `MemoryAccessViolation`;
    /// * If the registers exceed the memory limit returns `MemoryAccessViolation`;
    /// * If returning the preempted value into the registers exceed the memory container it returns
    ///   `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + storage_remove_base + storage_remove_key_byte * num_key_bytes + storage_remove_ret_value_byte * num_value_bytes
    /// + cost to read the key + cost to write the value`.
    pub fn storage_remove(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        unsafe { self.chain.storage_remove(key_len, key_ptr, register_id) }
    }
    /// Checks if there is a key-value pair.
    /// * If key is used returns `1`, even if the value is zero bytes;
    /// * Otherwise returns `0`.
    ///
    /// # Errors
    ///
    /// If `key_len + key_ptr` exceeds the memory container it returns `MemoryAccessViolation`.
    ///
    /// # Cost
    ///
    /// `base + storage_has_key_base + storage_has_key_byte * num_bytes + cost of reading key`
    pub fn storage_has_key(&mut self, key_len: u64, key_ptr: u64) -> u64 {
        unsafe { self.chain.storage_has_key(key_len, key_ptr) }
    }

    /**
     * Utilities
     */

    ///Computes the outcome of execution.
    pub fn outcome(&self) -> JsValue {
        let res = self.chain.outcome();
        let outcome = _VMOutcome {
            balance1: (res.balance >> 64) as u64,
            balance2: ((res.balance << 64) >> 64) as u64,
            storage_usage: res.storage_usage,
            return_data: res.return_data,
            burnt_gas: res.burnt_gas,
            used_gas: res.used_gas,
            logs: res.logs,
        };
        serde_wasm_bindgen::to_value(&outcome).unwrap()
    }
}

#[derive(Serialize)]
pub struct _VMOutcome {
    pub balance1: u64,
    pub balance2: u64,
    pub storage_usage: StorageUsage,
    pub return_data: ReturnData,
    pub burnt_gas: Gas,
    pub used_gas: Gas,
    pub logs: Vec<String>,
}
