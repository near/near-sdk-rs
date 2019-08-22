//#[cfg(not(feature = "env_test"))]
pub mod sys {
    extern "C" {
        // #############
        // # Registers #
        // #############
        pub fn read_register(register_id: u64, ptr: u64);
        pub fn register_len(register_id: u64) -> u64;
        // ###############
        // # Context API #
        // ###############
        pub fn current_account_id(register_id: u64);
        pub fn signer_account_id(register_id: u64);
        pub fn signer_account_pk(register_id: u64);
        pub fn predecessor_account_id(register_id: u64);
        pub fn input(register_id: u64);
        pub fn block_index() -> u64;
        pub fn storage_usage() -> u64;
        // #################
        // # Economics API #
        // #################
        pub fn account_balance(balance_ptr: u64);
        pub fn attached_deposit(balance_ptr: u64);
        pub fn prepaid_gas() -> u64;
        pub fn used_gas() -> u64;
        // ############
        // # Math API #
        // ############
        pub fn random_seed(register_id: u64);
        pub fn sha256(value_len: u64, value_ptr: u64, register_id: u64);
        // #####################
        // # Miscellaneous API #
        // #####################
        pub fn value_return(value_len: u64, value_ptr: u64);
        pub fn panic();
        pub fn log_utf8(len: u64, ptr: u64);
        pub fn log_utf16(len: u64, ptr: u64);
        // ################
        // # Promises API #
        // ################
        pub fn promise_create(
            account_id_len: u64,
            account_id_ptr: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) -> u64;
        pub fn promise_then(
            promise_index: u64,
            account_id_len: u64,
            account_id_ptr: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) -> u64;
        pub fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64;
        pub fn promise_results_count() -> u64;
        pub fn promise_result(result_idx: u64, register_id: u64) -> u64;
        pub fn promise_return(promise_id: u64);
        // ###############
        // # Storage API #
        // ###############
        pub fn storage_write(
            key_len: u64,
            key_ptr: u64,
            value_len: u64,
            value_ptr: u64,
            register_id: u64,
        ) -> u64;
        pub fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64;
        pub fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64;
        pub fn storage_has_key(key_len: u64, key_ptr: u64) -> u64;
        pub fn storage_iter_prefix(prefix_len: u64, prefix_ptr: u64) -> u64;
        pub fn storage_iter_range(
            start_len: u64,
            start_ptr: u64,
            end_len: u64,
            end_ptr: u64,
        ) -> u64;
        pub fn storage_iter_next(
            iterator_id: u64,
            key_register_id: u64,
            value_register_id: u64,
        ) -> u64;
    }
}
