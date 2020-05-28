#[cfg(target_arch = "wasm32")]
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
        pub fn block_timestamp() -> u64;
        pub fn epoch_height() -> u64;
        pub fn storage_usage() -> u64;
        // #################
        // # Economics API #
        // #################
        pub fn account_balance(balance_ptr: u64);
        pub fn account_locked_balance(balance_ptr: u64);
        pub fn attached_deposit(balance_ptr: u64);
        pub fn prepaid_gas() -> u64;
        pub fn used_gas() -> u64;
        // ############
        // # Math API #
        // ############
        pub fn random_seed(register_id: u64);
        pub fn sha256(value_len: u64, value_ptr: u64, register_id: u64);
        pub fn keccak256(value_len: u64, value_ptr: u64, register_id: u64);
        pub fn keccak512(value_len: u64, value_ptr: u64, register_id: u64);
        // #####################
        // # Miscellaneous API #
        // #####################
        pub fn value_return(value_len: u64, value_ptr: u64);
        pub fn panic();
        pub fn panic_utf8(len: u64, ptr: u64);
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
        pub fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64;
        pub fn promise_batch_then(
            promise_index: u64,
            account_id_len: u64,
            account_id_ptr: u64,
        ) -> u64;
        // #######################
        // # Promise API actions #
        // #######################
        pub fn promise_batch_action_create_account(promise_index: u64);
        pub fn promise_batch_action_deploy_contract(
            promise_index: u64,
            code_len: u64,
            code_ptr: u64,
        );
        pub fn promise_batch_action_function_call(
            promise_index: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        );
        pub fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64);
        pub fn promise_batch_action_stake(
            promise_index: u64,
            amount_ptr: u64,
            public_key_len: u64,
            public_key_ptr: u64,
        );
        pub fn promise_batch_action_add_key_with_full_access(
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
            nonce: u64,
        );
        pub fn promise_batch_action_add_key_with_function_call(
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
            nonce: u64,
            allowance_ptr: u64,
            receiver_id_len: u64,
            receiver_id_ptr: u64,
            method_names_len: u64,
            method_names_ptr: u64,
        );
        pub fn promise_batch_action_delete_key(
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
        );
        pub fn promise_batch_action_delete_account(
            promise_index: u64,
            beneficiary_id_len: u64,
            beneficiary_id_ptr: u64,
        );
        // #######################
        // # Promise API results #
        // #######################
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
        // ###############
        // # Validator API #
        // ###############
        pub fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64);
        pub fn validator_total_stake(stake_ptr: u64);
    }
}
