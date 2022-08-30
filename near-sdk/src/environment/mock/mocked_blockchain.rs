use super::{Receipt, SdkExternal};
use crate::mock::VmAction;
use crate::test_utils::VMContextBuilder;
use crate::types::{Balance, PromiseResult};
use crate::{Gas, RuntimeFeesConfig};
use crate::{PublicKey, VMContext};
use near_crypto::PublicKey as VmPublicKey;
use near_primitives::transaction::Action as PrimitivesAction;
use near_vm_logic::mocks::mock_memory::MockedMemory;
use near_vm_logic::types::PromiseResult as VmPromiseResult;
use near_vm_logic::{External, MemoryLike, VMConfig, VMLogic};
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

impl Default for MockedBlockchain {
    fn default() -> Self {
        MockedBlockchain::new(
            VMContextBuilder::new().build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            vec![],
            Default::default(),
            Default::default(),
            None,
        )
    }
}

struct LogicFixture {
    ext: Box<SdkExternal>,
    memory: Box<dyn MemoryLike>,
    #[allow(clippy::box_collection)]
    promise_results: Box<Vec<VmPromiseResult>>,
    config: Box<VMConfig>,
    fees_config: Box<RuntimeFeesConfig>,
}

impl MockedBlockchain {
    pub fn new(
        context: VMContext,
        config: VMConfig,
        fees_config: RuntimeFeesConfig,
        promise_results: Vec<PromiseResult>,
        storage: HashMap<Vec<u8>, Vec<u8>>,
        validators: HashMap<String, Balance>,
        memory_opt: Option<Box<dyn MemoryLike>>,
    ) -> Self {
        let mut ext = Box::new(SdkExternal::new());
        let context = sdk_context_to_vm_context(context);
        ext.fake_trie = storage;
        ext.validators = validators.into_iter().map(|(k, v)| (k.parse().unwrap(), v)).collect();
        let memory = memory_opt.unwrap_or_else(|| Box::new(MockedMemory {}));
        let promise_results = Box::new(promise_results.into_iter().map(From::from).collect());
        let config = Box::new(config);
        let fees_config = Box::new(fees_config);

        let mut logic_fixture = LogicFixture { ext, memory, promise_results, config, fees_config };

        let logic = unsafe {
            VMLogic::new_with_protocol_version(
                &mut *(logic_fixture.ext.as_mut() as *mut dyn External),
                context,
                &*(logic_fixture.config.as_mut() as *const VMConfig),
                &*(logic_fixture.fees_config.as_mut() as *const RuntimeFeesConfig),
                &*(logic_fixture.promise_results.as_ref().as_slice() as *const [VmPromiseResult]),
                &mut *(logic_fixture.memory.as_mut() as *mut dyn MemoryLike),
                u32::MAX,
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
        self.logic
            .borrow()
            .action_receipts()
            .iter()
            .map(|(receiver, receipt)| {
                let actions = receipt.actions.iter().map(action_to_sdk_action).collect();
                Receipt { receiver_id: receiver.as_str().parse().unwrap(), actions }
            })
            .collect()
    }

    pub fn gas(&mut self, gas_amount: u32) {
        self.logic.borrow_mut().gas(gas_amount).unwrap()
    }

    /// Returns logs created so far by the runtime.
    pub fn logs(&self) -> Vec<String> {
        self.logic.borrow().logs().to_vec()
    }
}

fn sdk_context_to_vm_context(context: VMContext) -> near_vm_logic::VMContext {
    near_vm_logic::VMContext {
        current_account_id: context.current_account_id.as_str().parse().unwrap(),
        signer_account_id: context.signer_account_id.as_str().parse().unwrap(),
        signer_account_pk: context.signer_account_pk.into_bytes(),
        predecessor_account_id: context.predecessor_account_id.as_str().parse().unwrap(),
        input: context.input,
        block_index: context.block_index,
        block_timestamp: context.block_timestamp,
        epoch_height: context.epoch_height,
        account_balance: context.account_balance,
        account_locked_balance: context.account_locked_balance,
        storage_usage: context.storage_usage,
        attached_deposit: context.attached_deposit,
        prepaid_gas: context.prepaid_gas.0,
        random_seed: context.random_seed.to_vec(),
        view_config: context.view_config,
        output_data_receivers: context
            .output_data_receivers
            .into_iter()
            .map(|a| a.as_str().parse().unwrap())
            .collect(),
    }
}

fn action_to_sdk_action(action: &PrimitivesAction) -> VmAction {
    match action {
        PrimitivesAction::CreateAccount(_) => VmAction::CreateAccount,
        PrimitivesAction::DeployContract(c) => VmAction::DeployContract { code: c.code.clone() },
        PrimitivesAction::FunctionCall(f) => VmAction::FunctionCall {
            function_name: f.method_name.clone(),
            args: f.args.clone(),
            gas: Gas(f.gas),
            deposit: f.deposit,
        },
        PrimitivesAction::Transfer(t) => VmAction::Transfer { deposit: t.deposit },
        PrimitivesAction::Stake(s) => {
            VmAction::Stake { stake: s.stake, public_key: pub_key_conversion(&s.public_key) }
        }
        PrimitivesAction::AddKey(k) => match &k.access_key.permission {
            near_primitives::account::AccessKeyPermission::FunctionCall(f) => {
                VmAction::AddKeyWithFunctionCall {
                    public_key: pub_key_conversion(&k.public_key),
                    nonce: k.access_key.nonce,
                    allowance: f.allowance,
                    receiver_id: f.receiver_id.parse().unwrap(),
                    function_names: f.method_names.clone(),
                }
            }
            near_primitives::account::AccessKeyPermission::FullAccess => {
                VmAction::AddKeyWithFullAccess {
                    public_key: pub_key_conversion(&k.public_key),
                    nonce: k.access_key.nonce,
                }
            }
        },
        PrimitivesAction::DeleteKey(k) => {
            VmAction::DeleteKey { public_key: pub_key_conversion(&k.public_key) }
        }
        PrimitivesAction::DeleteAccount(a) => {
            VmAction::DeleteAccount { beneficiary_id: a.beneficiary_id.parse().unwrap() }
        }
    }
}

fn pub_key_conversion(key: &VmPublicKey) -> PublicKey {
    // Hack by serializing and deserializing the key. This format should be consistent.
    String::from(key).parse().unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
mod mock_chain {
    use near_vm_logic::{VMLogic, VMLogicError};

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
