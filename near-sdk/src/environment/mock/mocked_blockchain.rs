use super::Receipt;
use crate::VMContext;
use crate::mock::MockAction;
use crate::test_utils::VMContextBuilder;
use crate::types::{NearToken, PromiseResult};
use near_parameters::{RuntimeConfigStore, RuntimeFeesConfig};
use near_primitives::gas::Gas;
use near_primitives_core::version::PROTOCOL_VERSION;
use near_vm_runner::logic::mocks::mock_external::MockedExternal;
use near_vm_runner::logic::types::{PromiseResult as VmPromiseResult, ReceiptIndex};
use near_vm_runner::logic::{ExecutionResultState, External, HostCtx, host};
use std::collections::HashMap;
use std::sync::Arc;

/// Mocked blockchain that can be used in the tests for the smart contracts.
///
/// Every host call lands in nearcore's own implementation of that host function
/// ([`near_vm_runner::logic::host`]), run against a [`HostCtx`] this type owns. Host errors
/// are unwrapped so they panic the unit test the way they would abort a contract on chain,
/// and tests can assert on the message.
pub struct MockedBlockchain {
    /// Borrows `fixture`; see [`MockedBlockchain::new`]. Declared first so it is dropped
    /// before what it borrows.
    ctx: HostCtx<'static>,
    fixture: HostFixture,
}

/// Placeholder for the `memory` parameter of [`MockedBlockchain::new`].
///
/// The host functions now take the guest memory as a plain `&mut [u8]`, so there is no
/// `MemoryLike` left to swap out. The parameter is kept so that existing `testing_env!`
/// and `MockedBlockchain::new(.., None)` call sites keep compiling.
#[derive(Default)]
pub struct MockedMemory;

/// The state [`HostCtx`] borrows for its whole life. Both fields are boxed so that moving
/// the [`MockedBlockchain`] around does not move what `ctx` points at.
struct HostFixture {
    ext: Box<MockedExternal>,
    context: Box<near_vm_runner::logic::VMContext>,
}

pub fn test_vm_config() -> near_parameters::vm::Config {
    let store = RuntimeConfigStore::test();
    let config = store.get_config(PROTOCOL_VERSION).wasm_config.as_ref().to_owned();
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

impl MockedBlockchain {
    pub fn new(
        context: VMContext,
        config: near_parameters::vm::Config,
        fees_config: RuntimeFeesConfig,
        promise_results: Vec<PromiseResult>,
        storage: HashMap<Vec<u8>, Vec<u8>>,
        validators: HashMap<String, NearToken>,
        memory: Option<MockedMemory>,
    ) -> Self {
        let _ = memory;
        let mut ext = Box::new(MockedExternal::new());
        let promise_results: Arc<[VmPromiseResult]> =
            promise_results.into_iter().map(Into::into).collect::<Vec<_>>().into();
        let context: Box<near_vm_runner::logic::VMContext> =
            Box::new(sdk_context_to_vm_context(context, promise_results));
        ext.fake_trie = storage;
        ext.validators = validators.into_iter().map(|(k, v)| (k.parse().unwrap(), v)).collect();
        let config = Arc::new(config);
        let result_state =
            ExecutionResultState::new(&context, context.make_gas_counter(&config), config.clone());

        let mut fixture = HostFixture { ext, context };

        // SAFETY: `ctx` only ever borrows the two boxes in `fixture`, which live as long as
        // this struct and are never reallocated. Field order drops `ctx` first, so the
        // borrows cannot outlive their targets. This is the same lifetime extension the
        // `VMLogic`-backed mock did.
        let ctx = unsafe {
            HostCtx::new(
                &mut *(fixture.ext.as_mut() as *mut dyn External),
                &*(fixture.context.as_mut() as *mut near_vm_runner::logic::VMContext),
                Arc::new(fees_config),
                result_state,
            )
        };

        Self { ctx, fixture }
    }

    pub fn take_storage(&mut self) -> HashMap<Vec<u8>, Vec<u8>> {
        std::mem::take(&mut self.fixture.ext.fake_trie)
    }

    /// Returns metadata about the receipts created
    pub fn created_receipts(&self) -> Vec<Receipt> {
        let action_log = &self.fixture.ext.action_log;
        let action_log: Vec<MockAction> =
            action_log.clone().into_iter().map(<MockAction as From<_>>::from).collect();
        let create_receipts: Vec<(usize, MockAction)> = action_log
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(_receipt_idx, action)| matches!(action, MockAction::CreateReceipt { .. }))
            .collect();

        create_receipts
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
            .collect()
    }

    pub fn gas(&mut self, gas_amount: u64) {
        host::burn_gas(&mut self.ctx, &mut [], gas_amount).unwrap()
    }

    /// Returns logs created so far by the runtime.
    pub fn logs(&self) -> Vec<String> {
        self.ctx.result_state().logs().to_vec()
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
        account_balance: context.account_balance,
        account_locked_balance: context.account_locked_balance,
        storage_usage: context.storage_usage,
        attached_deposit: context.attached_deposit,
        prepaid_gas: Gas::from_gas(context.prepaid_gas.as_gas()),
        random_seed: context.random_seed.to_vec(),
        view_config: context.view_config,
        output_data_receivers: context
            .output_data_receivers
            .into_iter()
            .map(|a| a.as_str().parse().unwrap())
            .collect(),
        promise_results,
        #[cfg(feature = "deterministic-account-ids")]
        refund_to_account_id: context.refund_to_account_id,
        #[cfg(not(feature = "deterministic-account-ids"))]
        refund_to_account_id: context.predecessor_account_id.as_str().parse().unwrap(),
        #[cfg(feature = "deterministic-account-ids")]
        account_contract: context.account_contract,
        #[cfg(not(feature = "deterministic-account-ids"))]
        account_contract: near_primitives::account::AccountContract::None,
    }
}

/// The `near-sys` host-function ABI, implemented for native unit tests.
///
/// `near-sdk`, `near-sdk-env`, `near-sdk-core` and `near-global-contracts` all call the
/// `extern "C"` symbols `near-sys` declares. On wasm those resolve to the runtime's imports;
/// here each one is defined as a `#[no_mangle]` shim that forwards to the very same function
/// the runtime uses, [`near_vm_runner::logic::host`].
#[cfg(not(target_arch = "wasm32"))]
mod mock_chain {
    use near_vm_runner::logic::errors::VMLogicError;
    use near_vm_runner::logic::{HostCtx, host};

    /// A scratch "guest memory" for one host-function call.
    ///
    /// The host functions address a single contiguous guest memory (`&mut [u8]`) by offset,
    /// while the shims below are handed raw pointers into this process. So each call gets a
    /// fresh buffer: [`Mem::copy_in`] copies an argument into it and returns the offset to
    /// pass in place of the pointer, and the handful of host functions that write to guest
    /// memory get a region from [`Mem::reserve_out`] that [`Mem::copy_out`] hands back to
    /// the caller's pointer afterwards.
    ///
    /// Dereferencing the raw pointers is sound for the same reason it was under the old
    /// `MemoryLike` mock: the only callers are `near_sdk::env` and its sibling crates, which
    /// always pass a pointer into a live Rust allocation together with its real length.
    #[derive(Default)]
    struct Mem(Vec<u8>);

    impl Mem {
        /// The buffer, in the shape the host functions take it.
        fn bytes(&mut self) -> &mut [u8] {
            &mut self.0
        }

        /// Copies the `len` bytes at native `ptr` into the buffer and returns the guest
        /// offset to pass in place of `ptr`.
        ///
        /// `len == u64::MAX` is the host's "read register `ptr` instead of memory" sentinel;
        /// it is passed through untouched. The three string functions (`log_utf8`,
        /// `log_utf16`, `panic_utf8`) read the same sentinel as "NUL-terminated string in
        /// guest memory" instead, which this cannot serve because the length is only known
        /// by scanning; `near_sdk::env` always passes a real length, so nothing in the SDK
        /// reaches that path.
        fn copy_in(&mut self, len: u64, ptr: u64) -> u64 {
            if len == u64::MAX {
                return ptr;
            }
            let offset = self.0.len() as u64;
            if len != 0 {
                self.0.extend_from_slice(unsafe {
                    std::slice::from_raw_parts(ptr as *const u8, len as usize)
                });
            }
            offset
        }

        /// Reserves `len` zero bytes for a host function to write into and returns the
        /// guest offset of the region.
        fn reserve_out(&mut self, len: u64) -> u64 {
            let offset = self.0.len() as u64;
            self.0.resize(self.0.len() + len as usize, 0);
            offset
        }

        /// Copies a region reserved by [`Self::reserve_out`] out to native `ptr`.
        fn copy_out(&self, offset: u64, len: u64, ptr: u64) {
            if len == 0 {
                return;
            }
            let src = &self.0[offset as usize..(offset + len) as usize];
            unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), ptr as *mut u8, len as usize) };
        }
    }

    /// Runs one host function against the mocked context and unwraps its error, so a host
    /// error panics the unit test the way it would abort a contract on chain.
    fn host_call<R>(
        f: impl FnOnce(&mut HostCtx<'static>, &mut Mem) -> Result<R, VMLogicError>,
    ) -> R {
        crate::mock::with_mocked_blockchain(|b| {
            let mut mem = Mem::default();
            f(&mut b.ctx, &mut mem).unwrap()
        })
    }

    /// A u128 argument (a balance) is read from guest memory as 16 little-endian bytes.
    const U128: u64 = 16;

    // ##############
    // # Registers  #
    // ##############

    #[unsafe(no_mangle)]
    extern "C-unwind" fn read_register(register_id: u64, ptr: u64) {
        host_call(|ctx, m| {
            // `read_register` carries no length, so the destination has to be sized first.
            // `register_len` is the only public way to ask and it charges one `base`, which
            // is the single place this mock burns gas the wasm path would not.
            let len = host::register_len(ctx, m.bytes(), register_id)?;
            // An unset register reports `u64::MAX`; let the real `read_register` below
            // raise `InvalidRegisterId` rather than reserving that much scratch.
            let len = if len == u64::MAX { 0 } else { len };
            let out = m.reserve_out(len);
            host::read_register(ctx, m.bytes(), register_id, out)?;
            m.copy_out(out, len, ptr);
            Ok(())
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn register_len(register_id: u64) -> u64 {
        host_call(|ctx, m| host::register_len(ctx, m.bytes(), register_id))
    }

    // ###############
    // # Context API #
    // ###############

    #[unsafe(no_mangle)]
    extern "C-unwind" fn current_account_id(register_id: u64) {
        host_call(|ctx, m| host::current_account_id(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn current_contract_code(register_id: u64) -> u64 {
        host_call(|ctx, m| host::current_contract_code(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn refund_to_account_id(register_id: u64) {
        host_call(|ctx, m| host::refund_to_account_id(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn signer_account_id(register_id: u64) {
        host_call(|ctx, m| host::signer_account_id(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn signer_account_pk(register_id: u64) {
        host_call(|ctx, m| host::signer_account_pk(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn predecessor_account_id(register_id: u64) {
        host_call(|ctx, m| host::predecessor_account_id(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn input(register_id: u64) {
        host_call(|ctx, m| host::input(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn chain_id(register_id: u64) {
        host_call(|ctx, m| host::chain_id(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn block_index() -> u64 {
        host_call(|ctx, m| host::block_index(ctx, m.bytes()))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn block_timestamp() -> u64 {
        host_call(|ctx, m| host::block_timestamp(ctx, m.bytes()))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn epoch_height() -> u64 {
        host_call(|ctx, m| host::epoch_height(ctx, m.bytes()))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn storage_usage() -> u64 {
        host_call(|ctx, m| host::storage_usage(ctx, m.bytes()))
    }

    // #################
    // # Economics API #
    // #################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn account_balance(balance_ptr: u64) {
        host_call(|ctx, m| {
            let out = m.reserve_out(U128);
            host::account_balance(ctx, m.bytes(), out)?;
            m.copy_out(out, U128, balance_ptr);
            Ok(())
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn account_locked_balance(balance_ptr: u64) {
        host_call(|ctx, m| {
            let out = m.reserve_out(U128);
            host::account_locked_balance(ctx, m.bytes(), out)?;
            m.copy_out(out, U128, balance_ptr);
            Ok(())
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn attached_deposit(balance_ptr: u64) {
        host_call(|ctx, m| {
            let out = m.reserve_out(U128);
            host::attached_deposit(ctx, m.bytes(), out)?;
            m.copy_out(out, U128, balance_ptr);
            Ok(())
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn prepaid_gas() -> u64 {
        host_call(|ctx, m| host::prepaid_gas(ctx, m.bytes()))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn used_gas() -> u64 {
        host_call(|ctx, m| host::used_gas(ctx, m.bytes()))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64) {
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            let out = m.reserve_out(U128);
            host::validator_stake(ctx, m.bytes(), account_id_len, account_id, out)?;
            m.copy_out(out, U128, stake_ptr);
            Ok(())
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn validator_total_stake(stake_ptr: u64) {
        host_call(|ctx, m| {
            let out = m.reserve_out(U128);
            host::validator_total_stake(ctx, m.bytes(), out)?;
            m.copy_out(out, U128, stake_ptr);
            Ok(())
        })
    }

    // ############
    // # Math API #
    // ############

    #[unsafe(no_mangle)]
    extern "C-unwind" fn random_seed(register_id: u64) {
        host_call(|ctx, m| host::random_seed(ctx, m.bytes(), register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn sha256(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::sha256(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn keccak256(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::keccak256(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn keccak512(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::keccak512(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn sha3_256(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::sha3_256(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn sha3_384(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::sha3_384(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn sha3_512(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::sha3_512(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn ripemd160(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::ripemd160(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn ecrecover(
        hash_len: u64,
        hash_ptr: u64,
        sig_len: u64,
        sig_ptr: u64,
        v: u64,
        malleability_flag: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let (hash, sig) = (m.copy_in(hash_len, hash_ptr), m.copy_in(sig_len, sig_ptr));
            host::ecrecover(
                ctx,
                m.bytes(),
                hash_len,
                hash,
                sig_len,
                sig,
                v,
                malleability_flag,
                register_id,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn ed25519_verify(
        signature_len: u64,
        signature_ptr: u64,
        message_len: u64,
        message_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let signature = m.copy_in(signature_len, signature_ptr);
            let message = m.copy_in(message_len, message_ptr);
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::ed25519_verify(
                ctx,
                m.bytes(),
                signature_len,
                signature,
                message_len,
                message,
                public_key_len,
                public_key,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn p256_verify(
        signature_len: u64,
        signature_ptr: u64,
        message_len: u64,
        message_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let signature = m.copy_in(signature_len, signature_ptr);
            let message = m.copy_in(message_len, message_ptr);
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::p256_verify(
                ctx,
                m.bytes(),
                signature_len,
                signature,
                message_len,
                message,
                public_key_len,
                public_key,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn ml_dsa_verify(
        signature_len: u64,
        signature_ptr: u64,
        message_len: u64,
        message_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let signature = m.copy_in(signature_len, signature_ptr);
            let message = m.copy_in(message_len, message_ptr);
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::ml_dsa_verify(
                ctx,
                m.bytes(),
                signature_len,
                signature,
                message_len,
                message,
                public_key_len,
                public_key,
            )
        })
    }

    // ##################
    // # Miscellaneous  #
    // ##################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn value_return(value_len: u64, value_ptr: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::value_return(ctx, m.bytes(), value_len, value)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn panic() -> ! {
        host_call(|ctx, m| host::panic(ctx, m.bytes()));
        unreachable!()
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn panic_utf8(len: u64, ptr: u64) -> ! {
        host_call(|ctx, m| {
            let msg = m.copy_in(len, ptr);
            host::panic_utf8(ctx, m.bytes(), len, msg)
        });
        unreachable!()
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn log_utf8(len: u64, ptr: u64) {
        host_call(|ctx, m| {
            let msg = m.copy_in(len, ptr);
            host::log_utf8(ctx, m.bytes(), len, msg)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn log_utf16(len: u64, ptr: u64) {
        host_call(|ctx, m| {
            let msg = m.copy_in(len, ptr);
            host::log_utf16(ctx, m.bytes(), len, msg)
        })
    }

    // ################
    // # Promises API #
    // ################

    #[unsafe(no_mangle)]
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
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            let function_name = m.copy_in(function_name_len, function_name_ptr);
            let arguments = m.copy_in(arguments_len, arguments_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_create(
                ctx,
                m.bytes(),
                account_id_len,
                account_id,
                function_name_len,
                function_name,
                arguments_len,
                arguments,
                amount,
                gas,
            )
        })
    }
    #[unsafe(no_mangle)]
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
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            let function_name = m.copy_in(function_name_len, function_name_ptr);
            let arguments = m.copy_in(arguments_len, arguments_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_then(
                ctx,
                m.bytes(),
                promise_index,
                account_id_len,
                account_id,
                function_name_len,
                function_name,
                arguments_len,
                arguments,
                amount,
                gas,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64 {
        host_call(|ctx, m| {
            // The host reads the indices as `promise_idx_count` little-endian `u64`s. On
            // overflow it raises `IntegerOverflow` before touching memory, so copy nothing.
            let len = promise_idx_count.checked_mul(size_of::<u64>() as u64).unwrap_or(0);
            let promise_idx = m.copy_in(len, promise_idx_ptr);
            host::promise_and(ctx, m.bytes(), promise_idx, promise_idx_count)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64 {
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            host::promise_batch_create(ctx, m.bytes(), account_id_len, account_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_then(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            host::promise_batch_then(ctx, m.bytes(), promise_index, account_id_len, account_id)
        })
    }

    // #######################
    // # Promise API actions #
    // #######################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_create_account(promise_index: u64) {
        host_call(|ctx, m| host::promise_batch_action_create_account(ctx, m.bytes(), promise_index))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_deploy_contract(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let code = m.copy_in(code_len, code_ptr);
            host::promise_batch_action_deploy_contract(
                ctx,
                m.bytes(),
                promise_index,
                code_len,
                code,
            )
        })
    }

    // #########################
    // # Global Contract API   #
    // #########################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_deploy_global_contract(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let code = m.copy_in(code_len, code_ptr);
            host::promise_batch_action_deploy_global_contract(
                ctx,
                m.bytes(),
                promise_index,
                code_len,
                code,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_deploy_global_contract_by_account_id(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let code = m.copy_in(code_len, code_ptr);
            host::promise_batch_action_deploy_global_contract_by_account_id(
                ctx,
                m.bytes(),
                promise_index,
                code_len,
                code,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_use_global_contract(
        promise_index: u64,
        code_hash_len: u64,
        code_hash_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let code_hash = m.copy_in(code_hash_len, code_hash_ptr);
            host::promise_batch_action_use_global_contract(
                ctx,
                m.bytes(),
                promise_index,
                code_hash_len,
                code_hash,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_use_global_contract_by_account_id(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            host::promise_batch_action_use_global_contract_by_account_id(
                ctx,
                m.bytes(),
                promise_index,
                account_id_len,
                account_id,
            )
        })
    }

    // ###########################
    // # Universal state init API #
    // ###########################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn universal_state_init_to_account_id(
        state_init_len: u64,
        state_init_ptr: u64,
        register_id: u64,
    ) {
        host_call(|ctx, m| {
            let state_init = m.copy_in(state_init_len, state_init_ptr);
            host::universal_state_init_to_account_id(
                ctx,
                m.bytes(),
                state_init_len,
                state_init,
                register_id,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_universal_state_init(
        promise_index: u64,
        state_init_len: u64,
        state_init_ptr: u64,
        amount_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let state_init = m.copy_in(state_init_len, state_init_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_universal_state_init(
                ctx,
                m.bytes(),
                promise_index,
                state_init_len,
                state_init,
                amount,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_set_refund_to(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            host::promise_set_refund_to(ctx, m.bytes(), promise_index, account_id_len, account_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_state_init(
        promise_index: u64,
        code_len: u64,
        code_ptr: u64,
        amount_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let code = m.copy_in(code_len, code_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_state_init(
                ctx,
                m.bytes(),
                promise_index,
                code_len,
                code,
                amount,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_state_init_by_account_id(
        promise_index: u64,
        account_id_len: u64,
        account_id_ptr: u64,
        amount_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let account_id = m.copy_in(account_id_len, account_id_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_state_init_by_account_id(
                ctx,
                m.bytes(),
                promise_index,
                account_id_len,
                account_id,
                amount,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn set_state_init_data_entry(
        promise_index: u64,
        action_index: u64,
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let (key, value) = (m.copy_in(key_len, key_ptr), m.copy_in(value_len, value_ptr));
            host::set_state_init_data_entry(
                ctx,
                m.bytes(),
                promise_index,
                action_index,
                key_len,
                key,
                value_len,
                value,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_function_call(
        promise_index: u64,
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
    ) {
        host_call(|ctx, m| {
            let function_name = m.copy_in(function_name_len, function_name_ptr);
            let arguments = m.copy_in(arguments_len, arguments_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_function_call(
                ctx,
                m.bytes(),
                promise_index,
                function_name_len,
                function_name,
                arguments_len,
                arguments,
                amount,
                gas,
            )
        })
    }
    #[unsafe(no_mangle)]
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
        host_call(|ctx, m| {
            let function_name = m.copy_in(function_name_len, function_name_ptr);
            let arguments = m.copy_in(arguments_len, arguments_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_function_call_weight(
                ctx,
                m.bytes(),
                promise_index,
                function_name_len,
                function_name,
                arguments_len,
                arguments,
                amount,
                gas,
                weight,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64) {
        host_call(|ctx, m| {
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_transfer(ctx, m.bytes(), promise_index, amount)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_stake(
        promise_index: u64,
        amount_ptr: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let amount = m.copy_in(U128, amount_ptr);
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::promise_batch_action_stake(
                ctx,
                m.bytes(),
                promise_index,
                amount,
                public_key_len,
                public_key,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_add_key_with_full_access(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        nonce: u64,
    ) {
        host_call(|ctx, m| {
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::promise_batch_action_add_key_with_full_access(
                ctx,
                m.bytes(),
                promise_index,
                public_key_len,
                public_key,
                nonce,
            )
        })
    }
    #[unsafe(no_mangle)]
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
        host_call(|ctx, m| {
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            let allowance = m.copy_in(U128, allowance_ptr);
            let receiver_id = m.copy_in(receiver_id_len, receiver_id_ptr);
            let function_names = m.copy_in(function_names_len, function_names_ptr);
            host::promise_batch_action_add_key_with_function_call(
                ctx,
                m.bytes(),
                promise_index,
                public_key_len,
                public_key,
                nonce,
                allowance,
                receiver_id_len,
                receiver_id,
                function_names_len,
                function_names,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_transfer_to_gas_key(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        amount_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            host::promise_batch_action_transfer_to_gas_key(
                ctx,
                m.bytes(),
                promise_index,
                public_key_len,
                public_key,
                amount,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_add_gas_key_with_full_access(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        num_nonces: u64,
    ) {
        host_call(|ctx, m| {
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::promise_batch_action_add_gas_key_with_full_access(
                ctx,
                m.bytes(),
                promise_index,
                public_key_len,
                public_key,
                num_nonces,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_add_gas_key_with_function_call(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
        num_nonces: u64,
        allowance_ptr: u64,
        receiver_id_len: u64,
        receiver_id_ptr: u64,
        method_names_len: u64,
        method_names_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            let allowance = m.copy_in(U128, allowance_ptr);
            let receiver_id = m.copy_in(receiver_id_len, receiver_id_ptr);
            let method_names = m.copy_in(method_names_len, method_names_ptr);
            host::promise_batch_action_add_gas_key_with_function_call(
                ctx,
                m.bytes(),
                promise_index,
                public_key_len,
                public_key,
                num_nonces,
                allowance,
                receiver_id_len,
                receiver_id,
                method_names_len,
                method_names,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_delete_key(
        promise_index: u64,
        public_key_len: u64,
        public_key_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let public_key = m.copy_in(public_key_len, public_key_ptr);
            host::promise_batch_action_delete_key(
                ctx,
                m.bytes(),
                promise_index,
                public_key_len,
                public_key,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_batch_action_delete_account(
        promise_index: u64,
        beneficiary_id_len: u64,
        beneficiary_id_ptr: u64,
    ) {
        host_call(|ctx, m| {
            let beneficiary_id = m.copy_in(beneficiary_id_len, beneficiary_id_ptr);
            host::promise_batch_action_delete_account(
                ctx,
                m.bytes(),
                promise_index,
                beneficiary_id_len,
                beneficiary_id,
            )
        })
    }

    // ######################
    // # Promise yield API  #
    // ######################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_yield_create(
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        gas: u64,
        gas_weight: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let function_name = m.copy_in(function_name_len, function_name_ptr);
            let arguments = m.copy_in(arguments_len, arguments_ptr);
            host::promise_yield_create(
                ctx,
                m.bytes(),
                function_name_len,
                function_name,
                arguments_len,
                arguments,
                gas,
                gas_weight,
                register_id,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_yield_resume(
        data_id_len: u64,
        data_id_ptr: u64,
        payload_len: u64,
        payload_ptr: u64,
    ) -> u32 {
        host_call(|ctx, m| {
            let data_id = m.copy_in(data_id_len, data_id_ptr);
            let payload = m.copy_in(payload_len, payload_ptr);
            host::promise_yield_resume(ctx, m.bytes(), data_id_len, data_id, payload_len, payload)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_yield_create_with_id(
        function_name_len: u64,
        function_name_ptr: u64,
        arguments_len: u64,
        arguments_ptr: u64,
        amount_ptr: u64,
        gas: u64,
        gas_weight: u64,
        yield_id_len: u64,
        yield_id_ptr: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let function_name = m.copy_in(function_name_len, function_name_ptr);
            let arguments = m.copy_in(arguments_len, arguments_ptr);
            let amount = m.copy_in(U128, amount_ptr);
            let yield_id = m.copy_in(yield_id_len, yield_id_ptr);
            host::promise_yield_create_with_id(
                ctx,
                m.bytes(),
                function_name_len,
                function_name,
                arguments_len,
                arguments,
                amount,
                gas,
                gas_weight,
                yield_id_len,
                yield_id,
            )
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_yield_resume_with_yield_id(
        yield_id_len: u64,
        yield_id_ptr: u64,
        payload_len: u64,
        payload_ptr: u64,
    ) -> u32 {
        host_call(|ctx, m| {
            let yield_id = m.copy_in(yield_id_len, yield_id_ptr);
            let payload = m.copy_in(payload_len, payload_ptr);
            host::promise_yield_resume_with_yield_id(
                ctx,
                m.bytes(),
                yield_id_len,
                yield_id,
                payload_len,
                payload,
            )
        })
    }

    // #######################
    // # Promise API results #
    // #######################

    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_results_count() -> u64 {
        host_call(|ctx, m| host::promise_results_count(ctx, m.bytes()))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_result(result_idx: u64, register_id: u64) -> u64 {
        host_call(|ctx, m| host::promise_result(ctx, m.bytes(), result_idx, register_id))
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn promise_return(promise_id: u64) {
        host_call(|ctx, m| host::promise_return(ctx, m.bytes(), promise_id))
    }

    // ###############
    // # Storage API #
    // ###############

    #[unsafe(no_mangle)]
    extern "C-unwind" fn storage_write(
        key_len: u64,
        key_ptr: u64,
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let (key, value) = (m.copy_in(key_len, key_ptr), m.copy_in(value_len, value_ptr));
            host::storage_write(ctx, m.bytes(), key_len, key, value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        host_call(|ctx, m| {
            let key = m.copy_in(key_len, key_ptr);
            host::storage_read(ctx, m.bytes(), key_len, key, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64 {
        host_call(|ctx, m| {
            let key = m.copy_in(key_len, key_ptr);
            host::storage_remove(ctx, m.bytes(), key_len, key, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn storage_has_key(key_len: u64, key_ptr: u64) -> u64 {
        host_call(|ctx, m| {
            let key = m.copy_in(key_len, key_ptr);
            host::storage_has_key(ctx, m.bytes(), key_len, key)
        })
    }

    // #############
    // # alt_bn128 #
    // #############

    #[unsafe(no_mangle)]
    extern "C-unwind" fn alt_bn128_g1_multiexp(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::alt_bn128_g1_multiexp(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn alt_bn128_g1_sum(value_len: u64, value_ptr: u64, register_id: u64) {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::alt_bn128_g1_sum(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn alt_bn128_pairing_check(value_len: u64, value_ptr: u64) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::alt_bn128_pairing_check(ctx, m.bytes(), value_len, value)
        })
    }

    // ###########
    // BLS12-381 #
    // ###########

    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_p1_sum(value_len: u64, value_ptr: u64, register_id: u64) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_p1_sum(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_p2_sum(value_len: u64, value_ptr: u64, register_id: u64) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_p2_sum(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_g1_multiexp(
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_g1_multiexp(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_g2_multiexp(
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_g2_multiexp(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_map_fp_to_g1(
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_map_fp_to_g1(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_map_fp2_to_g2(
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_map_fp2_to_g2(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_pairing_check(value_len: u64, value_ptr: u64) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_pairing_check(ctx, m.bytes(), value_len, value)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_p1_decompress(
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_p1_decompress(ctx, m.bytes(), value_len, value, register_id)
        })
    }
    #[unsafe(no_mangle)]
    extern "C-unwind" fn bls12381_p2_decompress(
        value_len: u64,
        value_ptr: u64,
        register_id: u64,
    ) -> u64 {
        host_call(|ctx, m| {
            let value = m.copy_in(value_len, value_ptr);
            host::bls12381_p2_decompress(ctx, m.bytes(), value_len, value, register_id)
        })
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
            [],
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
    #[cfg(feature = "deterministic-account-ids")]
    mod deterministic_account_ids {
        use crate::{
            GlobalContractId,
            state_init::{StateInit, StateInitV1},
        };

        use super::*;
        use near_account_id::{AccountId, AccountIdRef};
        use near_primitives::{account::AccountContract, hash::CryptoHash};
        use std::collections::BTreeMap;

        #[test]
        fn test_refund_to_contract_code() {
            let james: AccountId = "james.near".parse().unwrap();
            let context = VMContextBuilder::new()
                .account_contract(AccountContract::Local(CryptoHash([1; 32])))
                .refund_to_account_id(james.clone())
                .build();

            testing_env!(context.clone());

            let current_code = env::current_contract_code();
            match current_code {
                crate::AccountContract::Local(code) => assert_eq!(code, [1; 32]),
                _ => panic!("Expected Local account contract"),
            }

            let refund_to_account_id = env::refund_to_account_id();
            assert_eq!(refund_to_account_id, james);

            let promise_index = env::promise_create(
                james.clone(),
                "stake",
                [],
                NearToken::from_millinear(1),
                NearGas::from_tgas(1),
            );

            env::promise_set_refund_to(promise_index, &"mike.near".parse().unwrap());

            let actions = get_created_receipts();
            assert_eq!(
                actions[0].actions[1],
                MockAction::SetRefundTo {
                    receipt_index: 0,
                    refund_to_account_id: "mike.near".parse().unwrap(),
                }
            );
        }

        #[test]
        fn test_current_contract_code_global_by_account_returns_referenced_account() {
            let provider: AccountId = "a.near".parse().unwrap();
            let context = VMContextBuilder::new()
                .current_account_id("b.near".parse().unwrap())
                .account_contract(AccountContract::GlobalByAccount(provider.clone()))
                .build();
            testing_env!(context);

            assert_eq!(
                env::current_contract_code(),
                crate::AccountContract::GlobalByAccount(provider.clone()),
                "current_contract_code() must return the referenced global-code provider \
         (a.near), not the executing account (b.near)"
            );

            assert_eq!(
                env::current_global_contract_id(),
                Some(GlobalContractId::AccountId(provider)),
                "current_global_contract_id() must map to the referenced provider account"
            );
        }

        #[test]
        fn test_deterministic_update() {
            let context = VMContextBuilder::new().build();

            testing_env!(context.clone());

            let promise_index = env::promise_create(
                "account.near".parse().unwrap(),
                "method",
                [],
                NearToken::from_millinear(1),
                NearGas::from_tgas(1),
            );

            let action_index = env::promise_batch_action_state_init(
                promise_index,
                [1; 32],
                NearToken::from_millinear(1),
            );

            env::set_state_init_data_entry(promise_index, action_index, b"key", b"value");
            env::set_state_init_data_entry(promise_index, action_index, b"key2", b"value2");

            let actions = get_created_receipts();
            let mut tree = BTreeMap::new();
            tree.insert(b"key".to_vec(), b"value".to_vec());
            tree.insert(b"key2".to_vec(), b"value2".to_vec());
            assert_eq!(actions.len(), 1);
            assert_eq!(
                actions[0].actions[1],
                MockAction::DeterministicStateInit {
                    receipt_index: 0,
                    state_init: StateInit::V1(StateInitV1 {
                        code: GlobalContractId::CodeHash([1; 32]),
                        data: tree,
                    }),
                    amount: NearToken::from_millinear(1),
                }
            );
        }

        #[test]
        fn test_deterministic_account_id() {
            let context = VMContextBuilder::new().build();

            testing_env!(context.clone());

            let promise_index = env::promise_create(
                "account.near".parse().unwrap(),
                "method",
                [],
                NearToken::from_millinear(1),
                NearGas::from_tgas(1),
            );

            let action_index = env::promise_batch_action_state_init_by_account_id(
                promise_index,
                AccountIdRef::new("account.near").unwrap(),
                NearToken::from_millinear(1),
            );

            env::set_state_init_data_entry(promise_index, action_index, b"key", b"value");
            env::set_state_init_data_entry(promise_index, action_index, b"key2", b"value2");

            let actions = get_created_receipts();
            let mut tree = BTreeMap::new();
            tree.insert(b"key".to_vec(), b"value".to_vec());
            tree.insert(b"key2".to_vec(), b"value2".to_vec());
            assert_eq!(actions.len(), 1);
            assert_eq!(
                actions[0].actions[1],
                MockAction::DeterministicStateInit {
                    receipt_index: 0,
                    state_init: StateInit::V1(StateInitV1 {
                        code: GlobalContractId::AccountId("account.near".parse().unwrap()),
                        data: tree,
                    }),
                    amount: NearToken::from_millinear(1),
                }
            );
        }
    }
}
