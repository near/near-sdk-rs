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
    #[allow(dead_code)]
    pub fn panic();
    pub fn panic_utf8(len: u64, ptr: u64);
    pub fn log_utf8(len: u64, ptr: u64);
    #[allow(dead_code)]
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
    pub fn promise_batch_then(promise_index: u64, account_id_len: u64, account_id_ptr: u64) -> u64;
    // #######################
    // # Promise API actions #
    // #######################
    pub fn promise_batch_action_create_account(promise_index: u64);
    pub fn promise_batch_action_deploy_contract(promise_index: u64, code_len: u64, code_ptr: u64);
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

#[cfg(not(target_arch = "wasm32"))]
mod mock_chain {
    use crate::{env::BLOCKCHAIN_INTERFACE, BlockchainInterface};

    const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";

    fn with_mock_interface<F, R>(f: F) -> R
    where
        F: FnOnce(&dyn BlockchainInterface) -> R,
    {
        BLOCKCHAIN_INTERFACE
            .with(|b| f(b.borrow().as_ref().expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR).as_ref()))
    }

    #[no_mangle]
    extern "C" fn read_register(register_id: u64, ptr: u64) {
        with_mock_interface(|b| unsafe { b.read_register(register_id, ptr) })
    }
    #[no_mangle]
    extern "C" fn register_len(register_id: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.register_len(register_id) })
    }
    #[no_mangle]
    extern "C" fn current_account_id(register_id: u64) {
        with_mock_interface(|b| unsafe { b.current_account_id(register_id) })
    }
    #[no_mangle]
    extern "C" fn signer_account_id(register_id: u64) {
        with_mock_interface(|b| unsafe { b.signer_account_id(register_id) })
    }
    #[no_mangle]
    extern "C" fn signer_account_pk(register_id: u64) {
        with_mock_interface(|b| unsafe { b.signer_account_pk(register_id) })
    }
    #[no_mangle]
    extern "C" fn predecessor_account_id(register_id: u64) {
        with_mock_interface(|b| unsafe { b.predecessor_account_id(register_id) })
    }
    #[no_mangle]
    extern "C" fn input(register_id: u64) {
        with_mock_interface(|b| unsafe { b.input(register_id) })
    }
    #[no_mangle]
    extern "C" fn block_index() -> u64 {
        with_mock_interface(|b| unsafe { b.block_index() })
    }
    #[no_mangle]
    extern "C" fn block_timestamp() -> u64 {
        with_mock_interface(|b| unsafe { b.block_timestamp() })
    }
    #[no_mangle]
    extern "C" fn epoch_height() -> u64 {
        with_mock_interface(|b| unsafe { b.epoch_height() })
    }
    #[no_mangle]
    extern "C" fn storage_usage() -> u64 {
        todo!()
    }
    #[no_mangle]
    extern "C" fn account_balance(balance_ptr: u64) {
        with_mock_interface(|b| unsafe { b.account_balance(balance_ptr) })
    }
    #[no_mangle]
    extern "C" fn account_locked_balance(balance_ptr: u64) {
        with_mock_interface(|b| unsafe { b.account_locked_balance(balance_ptr) })
    }
    #[no_mangle]
    extern "C" fn attached_deposit(balance_ptr: u64) {
        with_mock_interface(|b| unsafe { b.attached_deposit(balance_ptr) })
    }
    #[no_mangle]
    extern "C" fn prepaid_gas() -> u64 {
        with_mock_interface(|b| unsafe { b.prepaid_gas() })
    }
    #[no_mangle]
    extern "C" fn used_gas() -> u64 {
        with_mock_interface(|b| unsafe { b.used_gas() })
    }
    #[no_mangle]
    extern "C" fn random_seed(register_id: u64) {
        with_mock_interface(|b| unsafe { b.random_seed(register_id) })
    }
    #[no_mangle]
    extern "C" fn sha256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| unsafe { b.sha256(value_len, value_ptr, register_id) })
    }
    #[no_mangle]
    extern "C" fn keccak256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| unsafe { b.keccak256(value_len, value_ptr, register_id) })
    }
    #[no_mangle]
    extern "C" fn keccak512(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| unsafe { b.keccak512(value_len, value_ptr, register_id) })
    }
    #[no_mangle]
    extern "C" fn value_return(value_len: u64, value_ptr: u64) {
        with_mock_interface(|b| unsafe { b.value_return(value_len, value_ptr) })
    }
    #[no_mangle]
    extern "C" fn panic() {
        with_mock_interface(|b| unsafe { b.panic() })
    }
    #[no_mangle]
    extern "C" fn panic_utf8(len: u64, ptr: u64) {
        with_mock_interface(|b| unsafe { b.panic_utf8(len, ptr) })
    }
    #[no_mangle]
    extern "C" fn log_utf8(len: u64, ptr: u64) {
        with_mock_interface(|b| unsafe { b.log_utf8(len, ptr) })
    }
    #[no_mangle]
    extern "C" fn log_utf16(len: u64, ptr: u64) {
        with_mock_interface(|b| unsafe { b.log_utf16(len, ptr) })
    }
    #[no_mangle]
    extern "C" fn promise_create(
        account_id_len: u64,
        account_id_ptr: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) -> u64 {
        with_mock_interface(|b| unsafe {
            b.promise_create(
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_then(
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
        with_mock_interface(|b| unsafe {
            b.promise_then(
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
        })
    }
    #[no_mangle]
    extern "C" fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.promise_and(promise_idx_ptr, promise_idx_count) })
    }
    #[no_mangle]
    extern "C" fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.promise_batch_create(account_id_len, account_id_ptr) })
    }
    #[no_mangle]
    extern "C" fn promise_batch_then(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        with_mock_interface(|b| unsafe {
            b.promise_batch_then(promise_index, account_id_len, account_id_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_create_account(promise_index: u64) {
        with_mock_interface(|b| unsafe { b.promise_batch_action_create_account(promise_index) })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_deploy_contract(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_deploy_contract(promise_index, code_len, code_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_function_call(
        promise_index: u64,
        method_name_len: u64,
        method_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_function_call(
                promise_index,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_transfer(promise_index, amount_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_stake(
        promise_index: u64,
        amount_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_stake(promise_index, amount_ptr, public_key_len, public_key_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_add_key_with_full_access(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_add_key_with_full_access(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_add_key_with_function_call(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
        allowance_ptr: u64,
        receiver_id_len: u64,
        receiver_id_ptr: u64,
        method_names_len: u64,
        method_names_ptr: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_add_key_with_function_call(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
                allowance_ptr,
                receiver_id_len,
                receiver_id_ptr,
                method_names_len,
                method_names_ptr,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_delete_key(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_delete_account(
        promise_index: u64,
        beneficiary_id_len: u64,
        beneficiary_id_ptr: u64,
    ) {
        with_mock_interface(|b| unsafe {
            b.promise_batch_action_delete_account(
                promise_index,
                beneficiary_id_len,
                beneficiary_id_ptr,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_results_count() -> u64 {
        with_mock_interface(|b| unsafe { b.promise_results_count() })
    }
    #[no_mangle]
    extern "C" fn promise_result(result_idx: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.promise_result(result_idx, register_id) })
    }
    #[no_mangle]
    extern "C" fn promise_return(promise_id: u64) {
        with_mock_interface(|b| unsafe { b.promise_return(promise_id) })
    }
    #[no_mangle]
    extern "C" fn storage_write(
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        with_mock_interface(|b| unsafe {
            b.storage_write(key_len, key_ptr, value_len, value_ptr, register_id)
        })
    }
    #[no_mangle]
    extern "C" fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.storage_read(key_len, key_ptr, register_id) })
    }
    #[no_mangle]
    extern "C" fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.storage_remove(key_len, key_ptr, register_id) })
    }
    #[no_mangle]
    extern "C" fn storage_has_key(key_len: u64, key_ptr: u64) -> u64 {
        with_mock_interface(|b| unsafe { b.storage_has_key(key_len, key_ptr) })
    }
    #[no_mangle]
    extern "C" fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
        with_mock_interface(|b| unsafe {
            b.validator_stake(account_id_len, account_id_ptr, stake_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn validator_total_stake(stake_ptr: u64) {
        with_mock_interface(|b| unsafe { b.validator_total_stake(stake_ptr) })
    }
}
