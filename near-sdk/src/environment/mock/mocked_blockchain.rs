use super::Receipt;
use crate::mock::MockAction;
// TODO replace with near_vm_logic::mocks::mock_memory::MockedMemory after updating version from 0.17
use crate::mock::mocked_memory::MockedMemory;
use crate::test_utils::VMContextBuilder;
use crate::types::{NearToken, PromiseResult};
use crate::VMContext;
use near_parameters::{RuntimeConfigStore, RuntimeFeesConfig};
use near_primitives_core::version::PROTOCOL_VERSION;
use near_vm_runner::logic::mocks::mock_external::MockedExternal;
use near_vm_runner::logic::types::{PromiseResult as VmPromiseResult, ReceiptIndex};
use near_vm_runner::logic::{External, MemoryLike, VMLogic};
use std::cell::RefCell;
use std::collections::HashMap;

/// Mocked blockchain that can be used in the tests for the smart contracts.
/// It implements `BlockchainInterface` by redirecting calls to `VMLogic`. It unwraps errors of
/// `VMLogic` to cause panic during the unit tests similarly to how errors of `VMLogic` would cause
/// the termination of guest program execution. Unit tests can even assert the expected error
/// message.
pub struct MockedBlockchain {
    logic: RefCell<VMLogic<'static>>,
    // We keep ownership over logic fixture so that references in `VMLogic` are valid.
    #[allow(dead_code)]
    logic_fixture: LogicFixture,
}

pub fn test_vm_config() -> near_parameters::vm::Config {
    let store = RuntimeConfigStore::test();
    let config = store.get_config(PROTOCOL_VERSION).wasm_config.clone();
    near_parameters::vm::Config {
        vm_kind: config.vm_kind.replace_with_wasmtime_if_unsupported(),
        ..config
    }
}

impl Default for MockedBlockchain {
    fn default() -> Self {
        MockedBlockchain::new(
            VMContextBuilder::new().build(),
            test_vm_config(),
            RuntimeFeesConfig::test(),
            vec![],
            Default::default(),
            Default::default(),
            None,
        )
    }
}

struct LogicFixture {
    ext: Box<MockedExternal>,
    memory: Box<dyn MemoryLike>,
    #[allow(clippy::box_collection)]
    promise_results: Box<Vec<VmPromiseResult>>,
    config: Box<near_parameters::vm::Config>,
    fees_config: Box<RuntimeFeesConfig>,
}

impl MockedBlockchain {
    pub fn new(
        context: VMContext,
        config: near_parameters::vm::Config,
        fees_config: RuntimeFeesConfig,
        promise_results: Vec<PromiseResult>,
        storage: HashMap<Vec<u8>, Vec<u8>>,
        validators: HashMap<String, NearToken>,
        memory_opt: Option<Box<dyn MemoryLike>>,
    ) -> Self {
        let mut ext = Box::new(MockedExternal::new());
        let context = sdk_context_to_vm_context(context);
        ext.fake_trie = storage;
        ext.validators =
            validators.into_iter().map(|(k, v)| (k.parse().unwrap(), v.as_yoctonear())).collect();
        let memory = memory_opt.unwrap_or_else(|| Box::<MockedMemory>::default());
        let promise_results = Box::new(promise_results.into_iter().map(From::from).collect());
        let config = Box::new(config);
        let fees_config = Box::new(fees_config);

        let mut logic_fixture = LogicFixture { ext, memory, promise_results, config, fees_config };

        let logic = unsafe {
            VMLogic::new(
                &mut *(logic_fixture.ext.as_mut() as *mut dyn External),
                context,
                &*(logic_fixture.config.as_mut() as *const near_parameters::vm::Config),
                &*(logic_fixture.fees_config.as_mut() as *const RuntimeFeesConfig),
                &*(logic_fixture.promise_results.as_ref().as_slice() as *const [VmPromiseResult]),
                &mut *(logic_fixture.memory.as_mut() as *mut dyn MemoryLike),
            )
        };

        let logic = RefCell::new(logic);
        Self { logic, logic_fixture }
    }

    pub fn take_storage(&mut self) -> HashMap<Vec<u8>, Vec<u8>> {
        std::mem::take(&mut self.logic_fixture.ext.fake_trie)
    }

    /// Returns metadata about the receipts created
    pub fn created_receipts(&self) -> Vec<Receipt> {
        let action_log = &self.logic_fixture.ext.action_log;
        let action_log: Vec<MockAction> =
            action_log.clone().into_iter().map(<MockAction as From<_>>::from).collect();
        let create_receipts: Vec<(usize, MockAction)> = action_log
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(_receipt_idx, action)| matches!(action, MockAction::CreateReceipt { .. }))
            .collect();

        let result = create_receipts
            .into_iter()
            .map(|(receipt_idx, create_receipt)| {
                let (receiver_id, receipt_indices) = match create_receipt {
                    MockAction::CreateReceipt { receiver_id, receipt_indices } => {
                        (receiver_id, receipt_indices)
                    }
                    _ => panic!("not a CreateReceipt action!"),
                };
                let actions: Vec<MockAction> = action_log
                    .iter()
                    .filter(|action| match action.receipt_index() {
                        None => false,
                        Some(action_receipt_idx) => {
                            action_receipt_idx == (receipt_idx as ReceiptIndex)
                        }
                    })
                    .cloned()
                    .collect();
                Receipt { receiver_id, actions, receipt_indices }
            })
            .collect();
        result
    }

    pub fn gas(&mut self, gas_amount: u32) {
        self.logic.borrow_mut().gas(gas_amount.into()).unwrap()
    }

    /// Returns logs created so far by the runtime.
    pub fn logs(&self) -> Vec<String> {
        self.logic.borrow().logs().to_vec()
    }
}

fn sdk_context_to_vm_context(context: VMContext) -> near_vm_runner::logic::VMContext {
    near_vm_runner::logic::VMContext {
        current_account_id: context.current_account_id.as_str().parse().unwrap(),
        signer_account_id: context.signer_account_id.as_str().parse().unwrap(),
        signer_account_pk: context.signer_account_pk.into_bytes(),
        predecessor_account_id: context.predecessor_account_id.as_str().parse().unwrap(),
        input: context.input,
        block_height: context.block_index,
        block_timestamp: context.block_timestamp,
        epoch_height: context.epoch_height,
        account_balance: context.account_balance.as_yoctonear(),
        account_locked_balance: context.account_locked_balance.as_yoctonear(),
        storage_usage: context.storage_usage,
        attached_deposit: context.attached_deposit.as_yoctonear(),
        prepaid_gas: context.prepaid_gas.as_gas(),
        random_seed: context.random_seed.to_vec(),
        view_config: context.view_config,
        output_data_receivers: context
            .output_data_receivers
            .into_iter()
            .map(|a| a.as_str().parse().unwrap())
            .collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod mock_chain {
    use near_vm_runner::logic::{errors::VMLogicError, VMLogic};
    fn with_mock_interface<F, R>(f: F) -> R
    where
        F: FnOnce(&mut VMLogic) -> Result<R, VMLogicError>,
    {
        crate::mock::with_mocked_blockchain(|b| f(&mut b.logic.borrow_mut()).unwrap())
    }

    #[no_mangle]
    extern "C" fn read_register(register_id: u64, ptr: u64) {
        with_mock_interface(|b| b.read_register(register_id, ptr))
    }
    #[no_mangle]
    extern "C" fn register_len(register_id: u64) -> u64 {
        with_mock_interface(|b| b.register_len(register_id))
    }
    #[no_mangle]
    extern "C" fn current_account_id(register_id: u64) {
        with_mock_interface(|b| b.current_account_id(register_id))
    }
    #[no_mangle]
    extern "C" fn signer_account_id(register_id: u64) {
        with_mock_interface(|b| b.signer_account_id(register_id))
    }
    #[no_mangle]
    extern "C" fn signer_account_pk(register_id: u64) {
        with_mock_interface(|b| b.signer_account_pk(register_id))
    }
    #[no_mangle]
    extern "C" fn predecessor_account_id(register_id: u64) {
        with_mock_interface(|b| b.predecessor_account_id(register_id))
    }
    #[no_mangle]
    extern "C" fn input(register_id: u64) {
        with_mock_interface(|b| b.input(register_id))
    }
    #[no_mangle]
    extern "C" fn block_index() -> u64 {
        with_mock_interface(|b| b.block_index())
    }
    #[no_mangle]
    extern "C" fn block_timestamp() -> u64 {
        with_mock_interface(|b| b.block_timestamp())
    }
    #[no_mangle]
    extern "C" fn epoch_height() -> u64 {
        with_mock_interface(|b| b.epoch_height())
    }
    #[no_mangle]
    extern "C" fn storage_usage() -> u64 {
        with_mock_interface(|b| b.storage_usage())
    }
    #[no_mangle]
    extern "C" fn account_balance(balance_ptr: u64) {
        with_mock_interface(|b| b.account_balance(balance_ptr))
    }
    #[no_mangle]
    extern "C" fn account_locked_balance(balance_ptr: u64) {
        with_mock_interface(|b| b.account_locked_balance(balance_ptr))
    }
    #[no_mangle]
    extern "C" fn attached_deposit(balance_ptr: u64) {
        with_mock_interface(|b| b.attached_deposit(balance_ptr))
    }
    #[no_mangle]
    extern "C" fn prepaid_gas() -> u64 {
        with_mock_interface(|b| b.prepaid_gas())
    }
    #[no_mangle]
    extern "C" fn used_gas() -> u64 {
        with_mock_interface(|b| b.used_gas())
    }
    #[no_mangle]
    extern "C" fn random_seed(register_id: u64) {
        with_mock_interface(|b| b.random_seed(register_id))
    }
    #[no_mangle]
    extern "C" fn sha256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.sha256(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn keccak256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.keccak256(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn keccak512(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.keccak512(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn ripemd160(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.ripemd160(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn ecrecover(
        hash_len: u64,
        hash_ptr: u64,
        sig_len: u64,
        sig_ptr: u64,
        v: u64,
        malleability_flag: u64,
        register_id: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.ecrecover(hash_len, hash_ptr, sig_len, sig_ptr, v, malleability_flag, register_id)
        })
    }
    #[no_mangle]
    extern "C" fn ed25519_verify(
        signature_len: u64,
        signature_ptr: u64,
        message_len: u64,
        message_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.ed25519_verify(
                signature_len,
                signature_ptr,
                message_len,
                message_ptr,
                public_key_len,
                public_key_ptr,
            )
        })
    }
    #[no_mangle]
    extern "C" fn value_return(value_len: u64, value_ptr: u64) {
        with_mock_interface(|b| b.value_return(value_len, value_ptr))
    }
    #[no_mangle]
    extern "C" fn panic() -> ! {
        with_mock_interface(|b| b.panic());
        unreachable!()
    }
    #[no_mangle]
    extern "C" fn panic_utf8(len: u64, ptr: u64) -> ! {
        with_mock_interface(|b| b.panic_utf8(len, ptr));
        unreachable!()
    }
    #[no_mangle]
    extern "C" fn log_utf8(len: u64, ptr: u64) {
        with_mock_interface(|b| b.log_utf8(len, ptr))
    }
    #[no_mangle]
    extern "C" fn log_utf16(len: u64, ptr: u64) {
        with_mock_interface(|b| b.log_utf16(len, ptr))
    }
    #[no_mangle]
    extern "C" fn promise_create(
        account_id_len: u64,
        account_id_ptr: u64,
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.promise_create(
                account_id_len,
                account_id_ptr,
                function_name_len,
                function_name_ptr,
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
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.promise_then(
                promise_index,
                account_id_len,
                account_id_ptr,
                function_name_len,
                function_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        with_mock_interface(|b| b.promise_and(promise_idx_ptr, promise_idx_count))
    }
    #[no_mangle]
    extern "C" fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64 {
        with_mock_interface(|b| b.promise_batch_create(account_id_len, account_id_ptr))
    }
    #[no_mangle]
    extern "C" fn promise_batch_then(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        with_mock_interface(|b| b.promise_batch_then(promise_index, account_id_len, account_id_ptr))
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_create_account(promise_index: u64) {
        with_mock_interface(|b| b.promise_batch_action_create_account(promise_index))
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_deploy_contract(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_deploy_contract(promise_index, code_len, code_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_function_call(
        promise_index: u64,
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_function_call(
                promise_index,
                function_name_len,
                function_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
            )
        })
    }

    #[no_mangle]
    extern "C" fn promise_batch_action_function_call_weight(
        promise_index: u64,
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
        weight: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_function_call_weight(
                promise_index,
                function_name_len,
                function_name_ptr,
                arguments_len,
                arguments_ptr,
                amount_ptr,
                gas,
                weight,
            )
        })
    }

    #[no_mangle]
    extern "C" fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64) {
        with_mock_interface(|b| b.promise_batch_action_transfer(promise_index, amount_ptr))
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_stake(
        promise_index: u64,
        amount_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| {
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
        with_mock_interface(|b| {
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
        function_names_len: u64,
        function_names_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_add_key_with_function_call(
                promise_index,
                public_key_len,
                public_key_ptr,
                nonce,
                allowance_ptr,
                receiver_id_len,
                receiver_id_ptr,
                function_names_len,
                function_names_ptr,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_delete_key(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr)
        })
    }
    #[no_mangle]
    extern "C" fn promise_batch_action_delete_account(
        promise_index: u64,
        beneficiary_id_len: u64,
        beneficiary_id_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_delete_account(
                promise_index,
                beneficiary_id_len,
                beneficiary_id_ptr,
            )
        })
    }
    #[no_mangle]
    extern "C" fn promise_results_count() -> u64 {
        with_mock_interface(|b| b.promise_results_count())
    }
    #[no_mangle]
    extern "C" fn promise_result(result_idx: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.promise_result(result_idx, register_id))
    }
    #[no_mangle]
    extern "C" fn promise_return(promise_id: u64) {
        with_mock_interface(|b| b.promise_return(promise_id))
    }
    #[no_mangle]
    extern "C" fn storage_write(
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        with_mock_interface(|b| {
            b.storage_write(key_len, key_ptr, value_len, value_ptr, register_id)
        })
    }
    #[no_mangle]
    extern "C" fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.storage_read(key_len, key_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.storage_remove(key_len, key_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn storage_has_key(key_len: u64, key_ptr: u64) -> u64 {
        with_mock_interface(|b| b.storage_has_key(key_len, key_ptr))
    }
    #[no_mangle]
    extern "C" fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
        with_mock_interface(|b| b.validator_stake(account_id_len, account_id_ptr, stake_ptr))
    }
    #[no_mangle]
    extern "C" fn validator_total_stake(stake_ptr: u64) {
        with_mock_interface(|b| b.validator_total_stake(stake_ptr))
    }
    #[no_mangle]
    extern "C" fn alt_bn128_g1_multiexp(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.alt_bn128_g1_multiexp(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn alt_bn128_g1_sum(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.alt_bn128_g1_sum(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C" fn alt_bn128_pairing_check(value_len: u64, value_ptr: u64) -> u64 {
        with_mock_interface(|b| b.alt_bn128_pairing_check(value_len, value_ptr))
    }
}
