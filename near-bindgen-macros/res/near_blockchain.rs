#[cfg(not(feature = "env_test"))]
pub mod near_blockchain {
    use super::sys;
    use near_bindgen::BlockchainInterface;
    /// Implementation of the blockchain interface that contracts actually use during the execution
    /// of the contract.
    pub struct NearBlockchain {}

    impl BlockchainInterface for NearBlockchain {
        unsafe fn read_register(&self, register_id: u64, ptr: u64) {
            unsafe { sys::read_register(register_id, ptr) }
        }

        unsafe fn register_len(&self, register_id: u64) -> u64 {
            unsafe { sys::register_len(register_id) }
        }

        unsafe fn current_account_id(&self, register_id: u64) {
            unsafe { sys::current_account_id(register_id) }
        }

        unsafe fn signer_account_id(&self, register_id: u64) {
            unsafe { sys::signer_account_id(register_id) }
        }

        unsafe fn signer_account_pk(&self, register_id: u64) {
            unsafe { sys::signer_account_pk(register_id) }
        }

        unsafe fn predecessor_account_id(&self, register_id: u64) {
            unsafe { sys::predecessor_account_id(register_id) }
        }

        unsafe fn input(&self, register_id: u64) {
            unsafe { sys::input(register_id) }
        }

        unsafe fn block_index(&self) -> u64 {
            unsafe { sys::block_index() }
        }

        unsafe fn storage_usage(&self) -> u64 {
            unsafe { sys::storage_usage() }
        }

        unsafe fn account_balance(&self, balance_ptr: u64) {
            unsafe { sys::account_balance(balance_ptr) }
        }

        unsafe fn attached_deposit(&self, balance_ptr: u64) {
            unsafe { sys::attached_deposit(balance_ptr) }
        }

        unsafe fn prepaid_gas(&self) -> u64 {
            unsafe { sys::prepaid_gas() }
        }

        unsafe fn used_gas(&self) -> u64 {
            unsafe { sys::used_gas() }
        }

        unsafe fn random_seed(&self, register_id: u64) {
            unsafe { sys::random_seed(register_id) }
        }

        unsafe fn sha256(&self, value_len: u64, value_ptr: u64, register_id: u64) {
            unsafe { sys::sha256(value_len, value_ptr, register_id) }
        }

        unsafe fn value_return(&self, value_len: u64, value_ptr: u64) {
            unsafe { sys::value_return(value_len, value_ptr) }
        }

        unsafe fn panic(&self) {
            unsafe { sys::panic() }
        }

        unsafe fn log_utf8(&self, len: u64, ptr: u64) {
            unsafe { sys::log_utf8(len, ptr) }
        }

        unsafe fn log_utf16(&self, len: u64, ptr: u64) {
            unsafe { sys::log_utf16(len, ptr) }
        }

        unsafe fn promise_create(
            &self,
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

        unsafe fn promise_then(
            &self,
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

        unsafe fn promise_and(&self, promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
            unsafe { sys::promise_and(promise_idx_ptr, promise_idx_count) }
        }

        unsafe fn promise_results_count(&self) -> u64 {
            unsafe { sys::promise_results_count() }
        }

        unsafe fn promise_result(&self, result_idx: u64, register_id: u64) -> u64 {
            unsafe { sys::promise_result(result_idx, register_id) }
        }

        unsafe fn promise_return(&self, promise_id: u64) {
            unsafe { sys::promise_return(promise_id) }
        }

        unsafe fn storage_write(
            &self,
            key_len: u64,
            key_ptr: u64,
            value_len: u64,
            value_ptr: u64,
            register_id: u64,
        ) -> u64 {
            unsafe { sys::storage_write(key_len, key_ptr, value_len, value_ptr, register_id) }
        }

        unsafe fn storage_read(&self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
            unsafe { sys::storage_read(key_len, key_ptr, register_id) }
        }

        unsafe fn storage_remove(&self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
            unsafe { sys::storage_remove(key_len, key_ptr, register_id) }
        }

        unsafe fn storage_has_key(&self, key_len: u64, key_ptr: u64) -> u64 {
            unsafe { sys::storage_has_key(key_len, key_ptr) }
        }

        unsafe fn storage_iter_prefix(&self, prefix_len: u64, prefix_ptr: u64) -> u64 {
            unsafe { sys::storage_iter_prefix(prefix_len, prefix_ptr) }
        }

        unsafe fn storage_iter_range(
            &self,
            start_len: u64,
            start_ptr: u64,
            end_len: u64,
            end_ptr: u64,
        ) -> u64 {
            unsafe { sys::storage_iter_range(start_len, start_ptr, end_len, end_ptr) }
        }

        unsafe fn storage_iter_next(
            &self,
            iterator_id: u64,
            key_register_id: u64,
            value_register_id: u64,
        ) -> u64 {
            unsafe { sys::storage_iter_next(iterator_id, key_register_id, value_register_id) }
        }
    }
}
