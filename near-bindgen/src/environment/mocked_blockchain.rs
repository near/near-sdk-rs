use crate::environment::blockchain_interface::BlockchainInterface;
use near_vm_logic::VMLogic;

/// Mocked blockchain that can be used in the tests for the smart contracts.
/// It implements `BlockchainInterface` by redirecting calls to `VMLogic`. It unwraps errors of
/// `VMLogic` to cause panic during the unit test similarly to how errors of `VMLogic` would cause
/// the termination of guest program execution. Unit tests can even assert the expected error
/// message.
pub struct MockedBlockchain<'a> {
    logic: VMLogic<'a>,
}

impl<'a> MockedBlockchain<'a> {
    pub fn new(logic: VMLogic<'a>) -> Self {
        Self { logic }
    }
}

impl<'a> BlockchainInterface for MockedBlockchain<'a> {
    fn read_register(&mut self, register_id: u64, ptr: u64) {
        self.logic.read_register(register_id, ptr).unwrap()
    }

    fn register_len(&mut self, register_id: u64) -> u64 {
        self.logic.register_len(register_id).unwrap()
    }

    fn current_account_id(&mut self, register_id: u64) {
        self.logic.current_account_id(register_id).unwrap()
    }

    fn signer_account_id(&mut self, register_id: u64) {
        self.logic.signer_account_id(register_id).unwrap()
    }

    fn signer_account_pk(&mut self, register_id: u64) {
        self.logic.signer_account_pk(register_id).unwrap()
    }

    fn predecessor_account_id(&mut self, register_id: u64) {
        self.logic.predecessor_account_id(register_id).unwrap()
    }

    fn input(&mut self, register_id: u64) {
        self.logic.input(register_id).unwrap()
    }

    fn block_index(&mut self) -> u64 {
        self.logic.block_index().unwrap()
    }

    fn storage_usage(&mut self) -> u64 {
        self.logic.storage_usage().unwrap()
    }

    fn account_balance(&mut self, balance_ptr: u64) {
        self.logic.account_balance(balance_ptr).unwrap()
    }

    fn attached_deposit(&mut self, balance_ptr: u64) {
        self.logic.attached_deposit(balance_ptr).unwrap()
    }

    fn prepaid_gas(&mut self) -> u64 {
        self.logic.prepaid_gas().unwrap()
    }

    fn used_gas(&mut self) -> u64 {
        self.logic.used_gas().unwrap()
    }

    fn random_seed(&mut self, register_id: u64) {
        self.logic.random_seed(register_id).unwrap()
    }

    fn sha256(&mut self, value_len: u64, value_ptr: u64, register_id: u64) {
        self.logic.sha256(value_len, value_ptr, register_id).unwrap()
    }

    fn value_return(&mut self, value_len: u64, value_ptr: u64) {
        self.logic.value_return(value_len, value_ptr).unwrap()
    }

    fn panic(&mut self) {
        self.logic.panic().unwrap()
    }

    fn log_utf8(&mut self, len: u64, ptr: u64) {
        self.logic.log_utf8(len, ptr).unwrap()
    }

    fn log_utf16(&mut self, len: u64, ptr: u64) {
        self.logic.log_utf16(len, ptr).unwrap()
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
        self.logic
            .promise_create(
                account_id_len,
                account_id_ptr,
                method_name_len,
                method_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
            .unwrap()
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
        self.logic
            .promise_then(
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
            .unwrap()
    }

    fn promise_and(&mut self, promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        self.logic.promise_and(promise_idx_ptr, promise_idx_count).unwrap()
    }

    fn promise_results_count(&mut self) -> u64 {
        self.logic.promise_results_count().unwrap()
    }

    fn promise_result(&mut self, result_idx: u64, register_id: u64) -> u64 {
        self.logic.promise_result(result_idx, register_id).unwrap()
    }

    fn promise_return(&mut self, promise_id: u64) {
        self.logic.promise_return(promise_id).unwrap()
    }

    fn storage_write(
        &mut self,
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        self.logic.storage_write(key_len, key_ptr, value_len, value_ptr, register_id).unwrap()
    }

    fn storage_read(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        self.logic.storage_read(key_len, key_ptr, register_id).unwrap()
    }

    fn storage_remove(&mut self, key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        self.logic.storage_remove(key_len, key_ptr, register_id).unwrap()
    }

    fn storage_has_key(&mut self, key_len: u64, key_ptr: u64) -> u64 {
        self.logic.storage_has_key(key_len, key_ptr).unwrap()
    }

    fn storage_iter_prefix(&mut self, prefix_len: u64, prefix_ptr: u64) -> u64 {
        self.logic.storage_iter_prefix(prefix_len, prefix_ptr).unwrap()
    }

    fn storage_iter_range(
        &mut self,
        start_len: u64,
        start_ptr: u64,
        end_len: u64,
        end_ptr: u64,
    ) -> u64 {
        self.logic.storage_iter_range(start_len, start_ptr, end_len, end_ptr).unwrap()
    }

    fn storage_iter_next(
        &mut self,
        iterator_id: u64,
        key_register_id: u64,
        value_register_id: u64,
    ) -> u64 {
        self.logic.storage_iter_next(iterator_id, key_register_id, value_register_id).unwrap()
    }
}
