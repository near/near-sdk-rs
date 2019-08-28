/// A low-level interface of either real or mocked blockchain that contract interacts with.
pub trait BlockchainInterface {
    // #############
    // # Registers #
    // #############
    fn read_register(&mut self, register_id: u64, ptr: u64);
    fn register_len(&mut self, register_id: u64) -> u64;
    // ###############
    // # Context API #
    // ###############
    fn current_account_id(&mut self, register_id: u64);
    fn signer_account_id(&mut self, register_id: u64);
    fn signer_account_pk(&mut self, register_id: u64);
    fn predecessor_account_id(&mut self, register_id: u64);
    fn input(&mut self, register_id: u64);
    fn block_index(&mut self) -> u64;
    fn storage_usage(&mut self) -> u64;
    // #################
    // # Economics API #
    // #################
    fn account_balance(&mut self, balance_ptr: u64);
    fn attached_deposit(&mut self, balance_ptr: u64);
    fn prepaid_gas(&mut self) -> u64;
    fn used_gas(&mut self) -> u64;
    // ############
    // # Math API #
    // ############
    fn random_seed(&mut self, register_id: u64);
    fn sha256(&mut self, value_len: u64, value_ptr: u64, register_id: u64);
    // #####################
    // # Miscellaneous API #
    // #####################
    fn value_return(&mut self, value_len: u64, value_ptr: u64);
    fn panic(&mut self);
    fn log_utf8(&mut self, len: u64, ptr: u64);
    fn log_utf16(&mut self, len: u64, ptr: u64);
    // ################
    // # Promises API #
    // ################
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
    ) -> u64;
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
    ) -> u64;
    fn promise_and(&mut self, promise_idx_ptr: u64, promise_idx_count: u64) -> u64;
    fn promise_results_count(&mut self) -> u64;
    fn promise_result(&mut self, result_idx: u64, register_id: u64) -> u64;
    fn promise_return(&mut self, promise_id: u64);
    // ###############
    // # Storage API #
    // ###############
    fn storage_write(
        &mut self,
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64;
    fn storage_read(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64;
    fn storage_remove(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64;
    fn storage_has_key(&mut self, key_len: u64, key_ptr: u64) -> u64;
    fn storage_iter_prefix(&mut self, prefix_len: u64, prefix_ptr: u64) -> u64;
    fn storage_iter_range(
        &mut self,
        start_len: u64,
        start_ptr: u64,
        end_len: u64,
        end_ptr: u64,
    ) -> u64;
    fn storage_iter_next(
        &mut self,
        iterator_id: u64,
        key_register_id: u64,
        value_register_id: u64,
    ) -> u64;
}
