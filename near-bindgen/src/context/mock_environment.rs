use crate::context::Environment;
use core::panicking::panic_fmt;
use near_vm_logic::types::{
    AccountId, Balance, BlockIndex, Gas, IteratorIndex, PromiseIndex, PromiseResult, PublicKey,
    StorageUsage,
};
use near_vm_logic::{VMContext, VMLogic};
use std::mem::size_of;

pub struct MockEnvironment<'a> {
    logic: VMLogic<'a>,
}

impl<'a> MockEnvironment<'a> {
    pub fn new(logic: VMLogic<'a>) -> Self {
        Self { logic }
    }
}

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

/// A simple macro helper to read blob value coming from host's method.
macro_rules! try_method_into_register {
    ( $method:ident ) => {{
        self.logic.$method(ATOMIC_OP_REGISTER).unwrap();
        self.read_register(ATOMIC_OP_REGISTER)
    }};
}

/// Same as `try_method_into_register` but expects the data.
macro_rules! method_into_register {
    ( $method:ident ) => {{
        try_method_into_register!($method).expect(REGISTER_EXPECTED_ERR)
    }};
}

impl Environment for MockEnvironment {
    fn read_register(&mut self, register_id: u64) -> Option<Vec<u8>> {
        let len = self.register_len(register_id)?;
        let mut res = vec![0u8; len as usize];
        self.logic.read_register(register_id, res.as_ptr() as u64).unwrap();
        Some(res)
    }

    fn register_len(&mut self, register_id: u64) -> Option<u64> {
        let len = self.logic.register_len(register_id).unwrap();
        if len == std::u64::MAX {
            None
        } else {
            Some(len)
        }
    }

    fn current_account_id(&mut self) -> AccountId {
        String::from_utf8(method_into_register!(current_account_id)).unwrap()
    }

    fn signer_account_id(&mut self) -> AccountId {
        String::from_utf8(method_into_register!(signer_account_id)).unwrap()
    }

    fn signer_account_pk(&mut self) -> PublicKey {
        method_into_register!(signer_account_pk)
    }

    fn predecessor_account_id(&mut self) -> String {
        String::from_utf8(method_into_register!(predecessor_account_id)).unwrap()
    }

    fn input(&mut self) -> Option<Vec<u8>> {
        try_method_into_register!(input)
    }

    fn block_index(&self) -> BlockIndex {
        self.logic.block_index().unwrap()
    }

    fn storage_usage(&self) -> StorageUsage {
        self.logic.storage_usage().unwrap()
    }

    fn account_balance(&mut self) -> Balance {
        let data = [0u8; size_of::<Balance>()];
        self.logic.account_balance(data.as_ptr() as u64).unwrap();
        Balance.from_le_bytes(data)
    }

    fn attached_deposit(&mut self) -> Balance {
        let data = [0u8; size_of::<Balance>()];
        self.logic.attached_deposit(data.as_ptr() as u64).unwrap();
        Balance.from_le_bytes(data)
    }

    fn prepaid_gas(&mut self) -> Gas {
        self.logic.prepaid_gas().unwrap()
    }

    fn used_gas(&mut self) -> Gas {
        self.logic.used_gas().unwrap()
    }

    fn random_seed(&mut self) -> Vec<u8> {
        method_into_register!(random_seed)
    }

    fn sha256(&mut self, value: &[u8]) -> Vec<u8> {
        self.logic.sha256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER).unwrap();
        self.read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)
    }

    fn promise_create(
        &mut self,
        account_id: AccountId,
        method_name: &[u8],
        arguments: &[u8],
        amount: Balance,
        gas: Gas,
    ) -> PromiseIndex {
        let account_id = account_id.as_bytes();
        self.logic
            .promise_create(
                account_id.len() as _,
                account_id.as_ptr() as _,
                method_name.len() as _,
                method_name.as_ptr() as _,
                arguments.len() as _,
                arguments.as_ptr() as _,
                &amount as *const Balance as _,
                gas,
            )
            .unwrap()
    }

    fn promise_then(
        &mut self,
        promise_idx: PromiseIndex,
        account_id: AccountId,
        method_name: &[u8],
        arguments: &[u8],
        amount: Balance,
        gas: Gas,
    ) -> PromiseIndex {
        let account_id = account_id.as_bytes();
        self.logic
            .promise_then(
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
            .unwrap()
    }

    fn promise_and(&mut self, promise_indices: &[PromiseIndex]) -> PromiseIndex {
        let mut data = vec![0u8; promise_indices.len() * size_of::<PromiseIndex>()];
        for i in 0..promise_indices.len() {
            data[i * size_of::<PromiseIndex>()..(i + 1) * size_of::<PromiseIndex>()]
                .copy_from_slice(&promise_indices[i].to_le_bytes());
        }
        self.logic.promise_and(data.as_ptr() as _, promise_indices.len() as _).unwrap()
    }

    fn promise_results_count(&self) -> u64 {
        self.logic.promise_results_count().unwrap()
    }

    fn promise_result(&mut self, result_idx: u64) -> PromiseResult {
        match self.logic.promise_result(result_idx, ATOMIC_OP_REGISTER).unwrap() {
            0 => PromiseResult::NotReady,
            1 => {
                let data = self
                    .read_register(ATOMIC_OP_REGISTER)
                    .expect("Promise result should've returned into register.");
                PromiseResult::Successful(data)
            }
            2 => PromiseResult::Failed,
            _ => panic!(RETURN_CODE_ERR),
        }
    }

    fn promise_return(&mut self, promise_idx: PromiseIndex) {
        self.logic.promise_return(promise_idx).unwrap()
    }

    fn value_return(&mut self, value: &[u8]) {
        self.logic.value_return(value.len() as _, value.as_ptr() as _).unwrap()
    }

    fn panic(&self) {
        self.logic.panic().unwrap()
    }

    fn log(&mut self, message: &[u8]) {
        self.logic.log_utf8(message.len() as _, message.as_ptr() as _).unwrap()
    }

    fn storage_write(&mut self, key: &[u8], value: &[u8]) -> bool {
        match self
            .logic
            .storage_write(
                key.len() as _,
                key.as_ptr() as _,
                value.len() as _,
                value.as_ptr() as _,
                EVICTED_REGISTER,
            )
            .unwrap()
        {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }

    fn storage_read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        match self
            .logic
            .storage_read(key.len() as _, key.as_ptr() as _, ATOMIC_OP_REGISTER)
            .unwrap()
        {
            0 => None,
            1 => Some(self.read_register(ATOMIC_OP_REGISTER).expect(REGISTER_EXPECTED_ERR)),
            _ => panic!(RETURN_CODE_ERR),
        }
    }

    fn storage_remove(&mut self, key: &[u8]) -> bool {
        match self
            .logic
            .storage_remove(key.len() as _, key.as_ptr() as _, EVICTED_REGISTER)
            .unwrap()
        {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }

    fn storage_get_evicted(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.read_register(EVICTED_REGISTER)
    }

    fn storage_has_key(&mut self, key: &[u8]) -> bool {
        match self.logic.storage_has_key(key.len() as _, key.as_ptr() as _).unwrap() {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }

    fn storage_iter_prefix(&mut self, prefix: &[u8]) -> IteratorIndex {
        self.logic.storage_iter_prefix(prefix.len() as _, prefix.as_ptr() as _).unwrap()
    }

    fn storage_iter_range(&mut self, start: &[u8], end: &[u8]) -> IteratorIndex {
        self.logic
            .storage_iter_range(
                start.len() as _,
                start.as_ptr() as _,
                end.len() as _,
                end.as_ptr() as _,
            )
            .unwrap()
    }

    fn storage_iter_next(&mut self, iterator_idx: IteratorIndex) -> bool {
        match self.logic.storage_iter_next(iterator_idx, KEY_REGISTER, VALUE_REGISTER).unwrap() {
            0 => false,
            1 => true,
            _ => panic!(RETURN_CODE_ERR),
        }
    }

    fn storage_iter_key_read(&mut self) -> Option<Vec<u8>> {
        self.read_register(KEY_REGISTER)
    }

    fn storage_iter_value_read(&mut self) -> Option<Vec<u8>> {
        self.read_register(VALUE_REGISTER)
    }
}
