//! The methods that are available by the smart contracts to call.
//! This is a safe wrapper around low-level `BlockchainInterface`.

use crate::environment::blockchain_interface::BlockchainInterface;
use near_vm_logic::types::{
    AccountId, Balance, BlockIndex, Gas, IteratorIndex, PromiseIndex, PromiseResult, PublicKey,
    StorageUsage,
};
use std::mem::size_of;

use std::cell::RefCell;

thread_local! {
/// Low-level blockchain interface wrapped by the environment.
/// It is static so that environment can be statically accessible. And it uses trait object so that
/// we can mock it with fake blockchain.
    pub static BLOCKCHAIN_INTERFACE: RefCell<Option<Box<dyn BlockchainInterface>>>
         = RefCell::new(None);
}

const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";

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

/// Key used to store the state of the contract.
const STATE_KEY: &[u8] = b"STATE";

/// A simple macro helper to read blob value coming from host's method.
macro_rules! try_method_into_register {
    ( $method:ident ) => {{
        BLOCKCHAIN_INTERFACE.with(|b| unsafe {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .$method(ATOMIC_OP_REGISTER);
        });
        read_register(ATOMIC_OP_REGISTER)
    }};
}

/// Same as `try_method_into_register` but expects the data.
macro_rules! method_into_register {
    ( $method:ident ) => {{
        try_method_into_register!($method).expect(REGISTER_EXPECTED_ERR)
    }};
}

pub fn set_blockchain_interface(blockchain_interface: Box<dyn BlockchainInterface>) {
    BLOCKCHAIN_INTERFACE.with(|b| {
        *b.borrow_mut() = Some(blockchain_interface);
    })
}

/// Reads the content of the `register_id`. If register is not used returns `None`.
pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    let len = register_len(register_id)?;
    let res = vec![0u8; len as usize];
    BLOCKCHAIN_INTERFACE.with(|b| unsafe {
        b.borrow()
            .as_ref()
            .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
            .read_register(register_id, res.as_ptr() as _)
    });
    Some(res)
}
/// Returns the size of the register. If register is not used returns `None`.
pub fn register_len(register_id: u64) -> Option<u64> {
    let len = BLOCKCHAIN_INTERFACE.with(|b| unsafe {
        b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).register_len(register_id)
    });
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
pub fn current_account_id() -> AccountId {
    String::from_utf8(method_into_register!(current_account_id)).unwrap()
}
/// The id of the account that either signed the original transaction or issued the initial
/// cross-contract call.
pub fn signer_account_id() -> AccountId {
    String::from_utf8(method_into_register!(signer_account_id)).unwrap()
}

/// The public key of the account that did the signing.
pub fn signer_account_pk() -> PublicKey {
    method_into_register!(signer_account_pk)
}
/// The id of the account that was the previous contract in the chain of cross-contract calls.
/// If this is the first contract, it is equal to `signer_account_id`.
pub fn predecessor_account_id() -> String {
    String::from_utf8(method_into_register!(predecessor_account_id)).unwrap()
}
/// The input to the contract call serialized as bytes. If input is not provided returns `None`.
pub fn input() -> Option<Vec<u8>> {
    try_method_into_register!(input)
}
/// Current block index.
pub fn block_index() -> BlockIndex {
    unsafe {
        BLOCKCHAIN_INTERFACE
            .with(|b| b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).block_index())
    }
}
/// Current block timestamp.
pub fn block_timestamp() -> u64 {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).block_timestamp()
        })
    }
}
/// Current total storage usage of this smart contract that this account would be paying for.
pub fn storage_usage() -> StorageUsage {
    unsafe {
        BLOCKCHAIN_INTERFACE
            .with(|b| b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).storage_usage())
    }
}

// #################
// # Economics API #
// #################
/// The balance attached to the given account. This includes the attached_deposit that was
/// attached to the transaction
pub fn account_balance() -> Balance {
    let data = [0u8; size_of::<Balance>()];
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .account_balance(data.as_ptr() as u64)
        })
    };
    Balance::from_le_bytes(data)
}
/// The balance that was attached to the call that will be immediately deposited before the
/// contract execution starts
pub fn attached_deposit() -> Balance {
    let data = [0u8; size_of::<Balance>()];
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .attached_deposit(data.as_ptr() as u64)
        })
    };
    Balance::from_le_bytes(data)
}
/// The amount of gas attached to the call that can be used to pay for the gas fees.
pub fn prepaid_gas() -> Gas {
    unsafe {
        BLOCKCHAIN_INTERFACE
            .with(|b| b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).prepaid_gas())
    }
}
/// The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
pub fn used_gas() -> Gas {
    unsafe {
        BLOCKCHAIN_INTERFACE
            .with(|b| b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).used_gas())
    }
}

// ############
// # Math API #
// ############
/// Get random seed from the register.
pub fn random_seed() -> Vec<u8> {
    method_into_register!(random_seed)
}
/// Hashes the random sequence of bytes using sha256.
pub fn sha256(value: &[u8]) -> Vec<u8> {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).sha256(
                value.len() as _,
                value.as_ptr() as _,
                ATOMIC_OP_REGISTER,
            )
        });
    };
    read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)
}

// ################
// # Promises API #
// ################
/// Creates a promise that will execute a method on account with given arguments and attaches
/// the given amount and gas.
pub fn promise_create(
    account_id: AccountId,
    method_name: &[u8],
    arguments: &[u8],
    amount: Balance,
    gas: Gas,
) -> PromiseIndex {
    let account_id = account_id.as_bytes();
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).promise_create(
                account_id.len() as _,
                account_id.as_ptr() as _,
                method_name.len() as _,
                method_name.as_ptr() as _,
                arguments.len() as _,
                arguments.as_ptr() as _,
                &amount as *const Balance as _,
                gas,
            )
        })
    }
}
/// Attaches the callback that is executed after promise pointed by `promise_idx` is complete.
pub fn promise_then(
    promise_idx: PromiseIndex,
    account_id: AccountId,
    method_name: &[u8],
    arguments: &[u8],
    amount: Balance,
    gas: Gas,
) -> PromiseIndex {
    let account_id = account_id.as_bytes();
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).promise_then(
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
        })
    }
}
/// Creates a new promise which completes when time all promises passed as arguments complete.
pub fn promise_and(promise_indices: &[PromiseIndex]) -> PromiseIndex {
    let mut data = vec![0u8; promise_indices.len() * size_of::<PromiseIndex>()];
    for i in 0..promise_indices.len() {
        data[i * size_of::<PromiseIndex>()..(i + 1) * size_of::<PromiseIndex>()]
            .copy_from_slice(&promise_indices[i].to_le_bytes());
    }
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_and(data.as_ptr() as _, promise_indices.len() as _)
        })
    }
}
pub fn promise_batch_create(account_id: &AccountId) -> PromiseIndex {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_create(account_id.len() as _, account_id.as_ptr() as _)
        })
    }
}
pub fn promise_batch_then(promise_index: PromiseIndex, account_id: &AccountId) -> PromiseIndex {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).promise_batch_then(
                promise_index,
                account_id.len() as _,
                account_id.as_ptr() as _,
            )
        })
    }
}
pub fn promise_batch_action_create_account(promise_index: PromiseIndex) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_create_account(promise_index)
        })
    }
}
pub fn promise_batch_action_deploy_contract(promise_index: u64, code: &[u8]) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_deploy_contract(
                    promise_index,
                    code.len() as _,
                    code.as_ptr() as _,
                )
        })
    }
}
pub fn promise_batch_action_function_call(
    promise_index: PromiseIndex,
    method_name: &[u8],
    arguments: &[u8],
    amount: Balance,
    gas: Gas,
) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_function_call(
                    promise_index,
                    method_name.len() as _,
                    method_name.as_ptr() as _,
                    arguments.len() as _,
                    arguments.as_ptr() as _,
                    &amount as *const Balance as _,
                    gas,
                )
        })
    }
}
pub fn promise_batch_action_transfer(promise_index: PromiseIndex, amount: Balance) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_transfer(promise_index, &amount as *const Balance as _)
        })
    }
}
pub fn promise_batch_action_stake(
    promise_index: PromiseIndex,
    amount: Balance,
    public_key: &PublicKey,
) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).promise_batch_action_stake(
                promise_index,
                &amount as *const Balance as _,
                public_key.len() as _,
                public_key.as_ptr() as _,
            )
        })
    }
}
pub fn promise_batch_action_add_key_with_full_access(
    promise_index: PromiseIndex,
    public_key: &PublicKey,
    nonce: u64,
) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_add_key_with_full_access(
                    promise_index,
                    public_key.len() as _,
                    public_key.as_ptr() as _,
                    nonce,
                )
        })
    }
}
pub fn promise_batch_action_add_key_with_function_call(
    promise_index: PromiseIndex,
    public_key: &PublicKey,
    nonce: u64,
    allowance: Balance,
    receiver_id: &AccountId,
    method_names: &[u8],
) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_add_key_with_function_call(
                    promise_index,
                    public_key.len() as _,
                    public_key.as_ptr() as _,
                    nonce,
                    &allowance as *const Balance as _,
                    receiver_id.len() as _,
                    receiver_id.as_ptr() as _,
                    method_names.len() as _,
                    method_names.as_ptr() as _,
                )
        })
    }
}
pub fn promise_batch_action_delete_key(promise_index: PromiseIndex, public_key: &PublicKey) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_delete_key(
                    promise_index,
                    public_key.len() as _,
                    public_key.as_ptr() as _,
                )
        })
    }
}
pub fn promise_batch_action_delete_account(
    promise_index: PromiseIndex,
    beneficiary_id: &AccountId,
) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_batch_action_delete_account(
                    promise_index,
                    beneficiary_id.len() as _,
                    beneficiary_id.as_ptr() as _,
                )
        })
    }
}

/// If the current function is invoked by a callback we can access the execution results of the
/// promises that caused the callback. This function returns the number of complete and
/// incomplete callbacks.
pub fn promise_results_count() -> u64 {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).promise_results_count()
        })
    }
}
/// If the current function is invoked by a callback we can access the execution results of the
/// promises that caused the callback.
pub fn promise_result(result_idx: u64) -> PromiseResult {
    match unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .promise_result(result_idx, ATOMIC_OP_REGISTER)
        })
    } {
        0 => PromiseResult::NotReady,
        1 => {
            let data = read_register(ATOMIC_OP_REGISTER)
                .expect("Promise result should've returned into register.");
            PromiseResult::Successful(data)
        }
        2 => PromiseResult::Failed,
        _ => panic!(RETURN_CODE_ERR),
    }
}
/// Consider the execution result of promise under `promise_idx` as execution result of this
/// function.
pub fn promise_return(promise_idx: PromiseIndex) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).promise_return(promise_idx)
        })
    }
}

// #####################
// # Miscellaneous API #
// #####################
/// Sets the blob of data as the return value of the contract.
pub fn value_return(value: &[u8]) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .value_return(value.len() as _, value.as_ptr() as _)
        })
    }
}
/// Terminates the execution of the program with the UTF-8 encoded message.
pub fn panic(message: &[u8]) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .panic_utf8(message.len() as _, message.as_ptr() as _)
        })
    }
}
/// Log the UTF-8 encodable message.
pub fn log(message: &[u8]) {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .log_utf8(message.len() as _, message.as_ptr() as _)
        })
    }
}

// ###############
// # Storage API #
// ###############
/// Writes key-value into storage.
/// If another key-value existed in the storage with the same key it returns `true`, otherwise `false`.
pub fn storage_write(key: &[u8], value: &[u8]) -> bool {
    match unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).storage_write(
                key.len() as _,
                key.as_ptr() as _,
                value.len() as _,
                value.as_ptr() as _,
                EVICTED_REGISTER,
            )
        })
    } {
        0 => false,
        1 => true,
        _ => panic!(RETURN_CODE_ERR),
    }
}
/// Reads the value stored under the given key.
pub fn storage_read(key: &[u8]) -> Option<Vec<u8>> {
    match unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).storage_read(
                key.len() as _,
                key.as_ptr() as _,
                ATOMIC_OP_REGISTER,
            )
        })
    } {
        0 => None,
        1 => Some(read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)),
        _ => panic!(RETURN_CODE_ERR),
    }
}
/// Removes the value stored under the given key.
/// If key-value existed returns `true`, otherwise `false`.
pub fn storage_remove(key: &[u8]) -> bool {
    match unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).storage_remove(
                key.len() as _,
                key.as_ptr() as _,
                EVICTED_REGISTER,
            )
        })
    } {
        0 => false,
        1 => true,
        _ => panic!(RETURN_CODE_ERR),
    }
}
/// Reads the most recent value that was evicted with `storage_write` or `storage_remove` command.
pub fn storage_get_evicted() -> Option<Vec<u8>> {
    read_register(EVICTED_REGISTER)
}
/// Checks if there is a key-value in the storage.
pub fn storage_has_key(key: &[u8]) -> bool {
    match unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .storage_has_key(key.len() as _, key.as_ptr() as _)
        })
    } {
        0 => false,
        1 => true,
        _ => panic!(RETURN_CODE_ERR),
    }
}
/// Creates an iterator that iterates key-values based on the prefix of the key.
pub fn storage_iter_prefix(prefix: &[u8]) -> IteratorIndex {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow()
                .as_ref()
                .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                .storage_iter_prefix(prefix.len() as _, prefix.as_ptr() as _)
        })
    }
}
/// Creates an iterator that iterates key-values in [start, end) interval.
pub fn storage_iter_range(start: &[u8], end: &[u8]) -> IteratorIndex {
    unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).storage_iter_range(
                start.len() as _,
                start.as_ptr() as _,
                end.len() as _,
                end.as_ptr() as _,
            )
        })
    }
}
/// Checks the next element of iterator progressing it. Returns `true` if the element is available.
/// Returns `false` if iterator has finished.
pub fn storage_iter_next(iterator_idx: IteratorIndex) -> bool {
    match unsafe {
        BLOCKCHAIN_INTERFACE.with(|b| {
            b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).storage_iter_next(
                iterator_idx,
                KEY_REGISTER,
                VALUE_REGISTER,
            )
        })
    } {
        0 => false,
        1 => true,
        _ => panic!(RETURN_CODE_ERR),
    }
}
/// Reads the key that iterator was pointing to.
pub fn storage_iter_key_read() -> Option<Vec<u8>> {
    read_register(KEY_REGISTER)
}
/// Reads the value that iterator was pointing to.
pub fn storage_iter_value_read() -> Option<Vec<u8>> {
    read_register(VALUE_REGISTER)
}

// ############################################
// # Saving and loading of the contract state #
// ############################################
/// Load the state of the given object.
pub fn state_read<T: borsh::BorshDeserialize>() -> Option<T> {
    storage_read(STATE_KEY)
        .map(|data| T::try_from_slice(&data).expect("Cannot deserialize the contract state."))
}

pub fn state_write<T: borsh::BorshSerialize>(state: &T) {
    let data = state.try_to_vec().expect("Cannot serialize the contract state.");
    storage_write(STATE_KEY, &data);
}
