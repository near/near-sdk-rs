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
use near_vm_runner::logic::{ExecutionResultState, External, MemoryLike, VMLogic};
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

/// Mocked blockchain that can be used in the tests for the smart contracts.
/// It implements `BlockchainInterface` by redirecting calls to `VMLogic`. It unwraps errors of
/// `VMLogic` to cause panic during the unit tests similarly to how errors of `VMLogic` would cause
/// the termination of guest program execution. Unit tests can even assert the expected error
/// message.
pub struct MockedBlockchain<Memory = MockedMemory>
where
    Memory: MemoryLike + Default + 'static,
{
    logic: RefCell<VMLogic<'static>>,
    // We keep ownership over logic fixture so that references in `VMLogic` are valid.
    #[allow(dead_code)]
    logic_fixture: LogicFixture,
    _memory: PhantomData<Memory>,
}

pub fn test_vm_config() -> near_parameters::vm::Config {
    let store = RuntimeConfigStore::test();
    let config = store.get_config(PROTOCOL_VERSION).wasm_config.as_ref().to_owned();
    near_parameters::vm::Config {
        vm_kind: config.vm_kind.replace_with_wasmtime_if_unsupported(),
        ..config
    }
}

impl<T> Default for MockedBlockchain<T>
where
    T: MemoryLike + Default + 'static,
{
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
    fees_config: Arc<RuntimeFeesConfig>,
    context: Box<near_vm_runner::logic::VMContext>,
}

impl<Memory> MockedBlockchain<Memory>
where
    Memory: MemoryLike + Default + 'static,
{
    pub fn new(
        context: VMContext,
        config: near_parameters::vm::Config,
        fees_config: RuntimeFeesConfig,
        promise_results: Vec<PromiseResult>,
        storage: HashMap<Vec<u8>, Vec<u8>>,
        validators: HashMap<String, NearToken>,
        memory: Option<Memory>,
    ) -> Self {
        let mut ext = Box::new(MockedExternal::new());
        let promise_results: Arc<[VmPromiseResult]> =
            promise_results.into_iter().map(Into::into).collect::<Vec<_>>().into();
        let context: Box<near_vm_runner::logic::VMContext> =
            Box::new(sdk_context_to_vm_context(context, promise_results));
        ext.fake_trie = storage;
        ext.validators =
            validators.into_iter().map(|(k, v)| (k.parse().unwrap(), v.as_yoctonear())).collect();
        let config = Arc::new(config);
        let fees_config = Arc::new(fees_config);
        let result_state =
            ExecutionResultState::new(&context, context.make_gas_counter(&config), config.clone());

        let mut logic_fixture = LogicFixture { ext, context, fees_config };

        let logic = unsafe {
            VMLogic::new(
                &mut *(logic_fixture.ext.as_mut() as *mut dyn External),
                &*(logic_fixture.context.as_mut() as *mut near_vm_runner::logic::VMContext),
                logic_fixture.fees_config.clone(),
                result_state,
                memory.unwrap_or_default(),
            )
        };

        let logic = RefCell::new(logic);
        Self { logic, logic_fixture, _memory: PhantomData }
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

fn sdk_context_to_vm_context(
    context: VMContext,
    promise_results: std::sync::Arc<[VmPromiseResult]>,
) -> near_vm_runner::logic::VMContext {
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
        promise_results,
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
    extern "C-unwind" fn read_register(register_id: u64, ptr: u64) {
        with_mock_interface(|b| b.read_register(register_id, ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn register_len(register_id: u64) -> u64 {
        with_mock_interface(|b| b.register_len(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn current_account_id(register_id: u64) {
        with_mock_interface(|b| b.current_account_id(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn signer_account_id(register_id: u64) {
        with_mock_interface(|b| b.signer_account_id(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn signer_account_pk(register_id: u64) {
        with_mock_interface(|b| b.signer_account_pk(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn predecessor_account_id(register_id: u64) {
        with_mock_interface(|b| b.predecessor_account_id(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn input(register_id: u64) {
        with_mock_interface(|b| b.input(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn block_index() -> u64 {
        with_mock_interface(|b| b.block_index())
    }
    #[no_mangle]
    extern "C-unwind" fn block_timestamp() -> u64 {
        with_mock_interface(|b| b.block_timestamp())
    }
    #[no_mangle]
    extern "C-unwind" fn epoch_height() -> u64 {
        with_mock_interface(|b| b.epoch_height())
    }
    #[no_mangle]
    extern "C-unwind" fn storage_usage() -> u64 {
        with_mock_interface(|b| b.storage_usage())
    }
    #[no_mangle]
    extern "C-unwind" fn account_balance(balance_ptr: u64) {
        with_mock_interface(|b| b.account_balance(balance_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn account_locked_balance(balance_ptr: u64) {
        with_mock_interface(|b| b.account_locked_balance(balance_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn attached_deposit(balance_ptr: u64) {
        with_mock_interface(|b| b.attached_deposit(balance_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn prepaid_gas() -> u64 {
        with_mock_interface(|b| b.prepaid_gas())
    }
    #[no_mangle]
    extern "C-unwind" fn used_gas() -> u64 {
        with_mock_interface(|b| b.used_gas())
    }
    #[no_mangle]
    extern "C-unwind" fn random_seed(register_id: u64) {
        with_mock_interface(|b| b.random_seed(register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn sha256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.sha256(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn keccak256(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.keccak256(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn keccak512(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.keccak512(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn ripemd160(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.ripemd160(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn ecrecover(
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
    extern "C-unwind" fn ed25519_verify(
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
    extern "C-unwind" fn value_return(value_len: u64, value_ptr: u64) {
        with_mock_interface(|b| b.value_return(value_len, value_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn panic() -> ! {
        with_mock_interface(|b| b.panic());
        unreachable!()
    }
    #[no_mangle]
    extern "C-unwind" fn panic_utf8(len: u64, ptr: u64) -> ! {
        with_mock_interface(|b| b.panic_utf8(len, ptr));
        unreachable!()
    }
    #[no_mangle]
    extern "C-unwind" fn log_utf8(len: u64, ptr: u64) {
        with_mock_interface(|b| b.log_utf8(len, ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn log_utf16(len: u64, ptr: u64) {
        with_mock_interface(|b| b.log_utf16(len, ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_create(
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
    extern "C-unwind" fn promise_then(
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
    extern "C-unwind" fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        with_mock_interface(|b| b.promise_and(promise_idx_ptr, promise_idx_count))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64 {
        with_mock_interface(|b| b.promise_batch_create(account_id_len, account_id_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_then(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        with_mock_interface(|b| b.promise_batch_then(promise_index, account_id_len, account_id_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_action_create_account(promise_index: u64) {
        with_mock_interface(|b| b.promise_batch_action_create_account(promise_index))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_action_deploy_contract(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_deploy_contract(promise_index, code_len, code_ptr)
        })
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_action_function_call(
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
    extern "C-unwind" fn promise_batch_action_function_call_weight(
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
    extern "C-unwind" fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64) {
        with_mock_interface(|b| b.promise_batch_action_transfer(promise_index, amount_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_action_stake(
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
    extern "C-unwind" fn promise_batch_action_add_key_with_full_access(
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
    extern "C-unwind" fn promise_batch_action_add_key_with_function_call(
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
    extern "C-unwind" fn promise_batch_action_delete_key(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        with_mock_interface(|b| {
            b.promise_batch_action_delete_key(promise_index, public_key_len, public_key_ptr)
        })
    }
    #[no_mangle]
    extern "C-unwind" fn promise_batch_action_delete_account(
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
    extern "C-unwind" fn promise_results_count() -> u64 {
        with_mock_interface(|b| b.promise_results_count())
    }
    #[no_mangle]
    extern "C-unwind" fn promise_result(result_idx: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.promise_result(result_idx, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn promise_return(promise_id: u64) {
        with_mock_interface(|b| b.promise_return(promise_id))
    }
    #[no_mangle]
    extern "C-unwind" fn storage_write(
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
    extern "C-unwind" fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.storage_read(key_len, key_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        with_mock_interface(|b| b.storage_remove(key_len, key_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn storage_has_key(key_len: u64, key_ptr: u64) -> u64 {
        with_mock_interface(|b| b.storage_has_key(key_len, key_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
        with_mock_interface(|b| b.validator_stake(account_id_len, account_id_ptr, stake_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn validator_total_stake(stake_ptr: u64) {
        with_mock_interface(|b| b.validator_total_stake(stake_ptr))
    }
    #[no_mangle]
    extern "C-unwind" fn alt_bn128_g1_multiexp(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.alt_bn128_g1_multiexp(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn alt_bn128_g1_sum(value_len: u64, value_ptr: u64, register_id: u64) {
        with_mock_interface(|b| b.alt_bn128_g1_sum(value_len, value_ptr, register_id))
    }
    #[no_mangle]
    extern "C-unwind" fn alt_bn128_pairing_check(value_len: u64, value_ptr: u64) -> u64 {
        with_mock_interface(|b| b.alt_bn128_pairing_check(value_len, value_ptr))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use near_gas::NearGas;
    use near_primitives::types::GasWeight;

    use crate::{
        env,
        test_utils::{accounts, get_created_receipts, get_logs},
        testing_env,
    };

    use super::*;

    #[test]
    fn test_mocked_blockchain_api() {
        let public_key: crate::types::PublicKey =
            "ed25519:H3C2AVAWKq5Qm7FkyDB5cHKcYKHgbiiB2BzX8DQX8CJ".parse().unwrap();
        let context = VMContextBuilder::new()
            .signer_account_id(accounts(0))
            .signer_account_pk(public_key.clone())
            .build();

        testing_env!(context.clone());
        assert_eq!(env::signer_account_id(), accounts(0));
        assert_eq!(env::signer_account_pk(), public_key);

        env::storage_write(b"smile", b"hello_worlds");
        assert_eq!(env::storage_read(b"smile").unwrap(), b"hello_worlds");
        assert!(env::storage_has_key(b"smile"));
        env::storage_remove(b"smile");
        assert!(!env::storage_has_key(b"smile"));

        let promise_index = env::promise_create(
            "account.near".parse().unwrap(),
            "method",
            &[],
            NearToken::from_millinear(1),
            NearGas::from_tgas(1),
        );

        env::promise_batch_action_stake(promise_index, NearToken::from_millinear(1), &public_key);

        env::log_str("logged");

        let logs = get_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], "logged");

        let actions = get_created_receipts();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].receiver_id.to_string(), "account.near");
        assert_eq!(actions[0].actions.len(), 2);
        assert_eq!(
            actions[0].actions[0],
            MockAction::FunctionCallWeight {
                receipt_index: 0,
                method_name: b"method".to_vec(),
                args: [].to_vec(),
                attached_deposit: NearToken::from_millinear(1),
                prepaid_gas: NearGas::from_tgas(1),
                gas_weight: GasWeight(0)
            }
        );

        assert_eq!(
            actions[0].actions[1],
            MockAction::Stake {
                receipt_index: 0,
                stake: NearToken::from_millinear(1),
                public_key: near_crypto::PublicKey::from_str(
                    "ed25519:H3C2AVAWKq5Qm7FkyDB5cHKcYKHgbiiB2BzX8DQX8CJ"
                )
                .unwrap()
            }
        );
    }
}
