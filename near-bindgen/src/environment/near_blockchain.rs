use crate::context::blockchain_interface::BlockchainInterface;
use crate::context::sys::sys;

/// Implementation of the blockchain interface that contracts actually use during the execution
/// of the contract.
pub struct NearBlockchain {}

impl BlockchainInterface for NearBlockchain {
    fn read_register(&mut self, register_id: u64, ptr: u64) {
        unsafe { sys::read_register(register_id, ptr) }
    }

    fn register_len(&mut self, register_id: u64) -> u64 {
        unsafe { sys::register_len(register_id) }
    }

    fn current_account_id(&mut self, register_id: u64) {
        unsafe { sys::current_account_id(register_id) }
    }

    fn signer_account_id(&mut self, register_id: u64) {
        unsafe { sys::signer_account_id(register_id) }
    }

    fn signer_account_pk(&mut self, register_id: u64) {
        unsafe { sys::signer_account_pk(register_id) }
    }

    fn predecessor_account_id(&mut self, register_id: u64) {
        unsafe { sys::predecessor_account_id(register_id) }
    }

    fn input(&mut self, register_id: u64) {
        unsafe { sys::input(register_id) }
    }

    fn block_index(&mut self) -> u64 {
        unsafe { sys::block_index() }
    }

    fn storage_usage(&mut self) -> u64 {
        unsafe { sys::storage_usage() }
    }

    fn account_balance(&mut self, balance_ptr: u64) {
        unsafe { sys::account_balance(balance_ptr) }
    }

    fn attached_deposit(&mut self, balance_ptr: u64) {
        unsafe { sys::attached_deposit(balance_ptr) }
    }

    fn prepaid_gas(&mut self) -> u64 {
        unsafe { sys::prepaid_gas() }
    }

    fn used_gas(&mut self) -> u64 {
        unsafe { sys::used_gas() }
    }

    fn random_seed(&mut self, register_id: u64) {
        unsafe { sys::random_seed(register_id) }
    }

    fn sha256(&mut self, value_len: u64, value_ptr: u64, register_id: u64) {
        unsafe { sys::sha256(value_len, value_ptr, register_id) }
    }

    fn value_return(&mut self, value_len: u64, value_ptr: u64) {
        unsafe { sys::value_return(value_len, value_ptr) }
    }

    fn panic(&mut self) {
        unsafe { sys::panic() }
    }

    fn log_utf8(&mut self, len: u64, ptr: u64) {
        unsafe { sys::log_utf8(len, ptr) }
    }

    fn log_utf16(&mut self, len: u64, ptr: u64) {
        unsafe { sys::log_utf16(len, ptr) }
    }

    fn promise_create(
        &mut self,
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
            sys::promise_create(
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

    fn promise_then(
        &mut self,
        promise_index: u64,
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
            sys::promise_then(
                promise_index,
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

    fn promise_and(&mut self, promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        unsafe { sys::promise_and(promise_idx_ptr, promise_idx_count) }
    }

    fn promise_results_count(&mut self) -> u64 {
        unsafe { sys::promise_results_count() }
    }

    fn promise_result(&mut self, result_idx: u64, register_id: u64) -> u64 {
        unsafe { sys::promise_result(result_idx, register_id) }
    }

    fn promise_return(&mut self, promise_id: u64) {
        unsafe { sys::promise_return(promise_id) }
    }

    fn storage_write(
        &mut self,
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        unsafe { sys::storage_write(key_len, key_ptr, value_len, value_ptr, register_id) }
    }

    fn storage_read(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        unsafe { sys::storage_read(key_len, key_ptr, register_id) }
    }

    fn storage_remove(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        unsafe { sys::storage_remove(key_len, key_ptr, register_id) }
    }

    fn storage_has_key(&mut self, key_len: u64, key_ptr: u64) -> u64 {
        unsafe { sys::storage_has_key(key_len, key_ptr) }
    }

    fn storage_iter_prefix(&mut self, prefix_len: u64, prefix_ptr: u64) -> u64 {
        unsafe { sys::storage_iter_prefix(prefix_len, prefix_ptr) }
    }

    fn storage_iter_range(
        &mut self,
        start_len: u64,
        start_ptr: u64,
        end_len: u64,
        end_ptr: u64,
    ) -> u64 {
        unsafe { sys::storage_iter_range(start_len, start_ptr, end_len, end_ptr) }
    }

    fn storage_iter_next(
        &mut self,
        iterator_id: u64,
        key_register_id: u64,
        value_register_id: u64,
    ) -> u64 {
        unsafe { sys::storage_iter_next(iterator_id, key_register_id, value_register_id) }
    }
}
