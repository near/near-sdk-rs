//! Link-time stubs for the `near-sys` host functions, for ABI generation only.
//!
//! `cargo near abi` builds the contract for the HOST target as a `cdylib`, then loads it
//! (`cargo-near-build`'s `abi::generate::dylib`) and calls its `__near_abi_*` exports to
//! read back each method's schema. Linking that `cdylib` has to resolve every `extern "C"`
//! symbol [`near_sys`] declares, because those are wasm imports the NEAR runtime supplies
//! on-chain and nothing supplies them off-chain.
//!
//! ELF and Mach-O leave undefined symbols in a shared object to be resolved on load, so
//! the link succeeds on Linux and macOS and nobody noticed. The MSVC linker resolves
//! everything up front and fails with `LNK2019: unresolved external symbol` once per
//! referenced host function, which is why ABI generation has never worked on a
//! `x86_64-pc-windows-msvc` host (#1221, and near/cargo-near#188).
//!
//! So this module defines each one. ABI generation only reads type information, so a
//! stub is never called; each panics rather than returning a plausible value, so a
//! future caller fails loudly instead of silently reading a zero as chain state.
//!
//! Unit tests need working host functions, not stubs, and get them from
//! `environment::mock::mock_chain`, which defines the same symbols against
//! `near_vm_runner`. Both definitions in one link would collide, so this module is
//! compiled only when `unit-testing` is off.
//!
//! Every symbol is defined here, including those `near-sys` declares behind a feature:
//! an unused definition is harmless, while a missing one is the link error above.
//! `stubs_cover_every_near_sys_host_function` keeps the two lists in step.

fn unavailable(name: &str) -> ! {
    panic!(
        "the NEAR host function `{name}` is not available outside the NEAR runtime; \
         this build of the contract exists only to generate its ABI"
    )
}

#[unsafe(no_mangle)]
extern "C-unwind" fn read_register(_register_id: u64, _ptr: u64) {
    unavailable("read_register")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn register_len(_register_id: u64) -> u64 {
    unavailable("register_len")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn write_register(_register_id: u64, _data_len: u64, _data_ptr: u64) {
    unavailable("write_register")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn current_account_id(_register_id: u64) {
    unavailable("current_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn current_contract_code(_register_id: u64) -> u64 {
    unavailable("current_contract_code")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn refund_to_account_id(_register_id: u64) {
    unavailable("refund_to_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn signer_account_id(_register_id: u64) {
    unavailable("signer_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn signer_account_pk(_register_id: u64) {
    unavailable("signer_account_pk")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn predecessor_account_id(_register_id: u64) {
    unavailable("predecessor_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn input(_register_id: u64) {
    unavailable("input")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn chain_id(_register_id: u64) {
    unavailable("chain_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn block_index() -> u64 {
    unavailable("block_index")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn block_timestamp() -> u64 {
    unavailable("block_timestamp")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn epoch_height() -> u64 {
    unavailable("epoch_height")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_usage() -> u64 {
    unavailable("storage_usage")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn account_balance(_balance_ptr: u64) {
    unavailable("account_balance")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn account_locked_balance(_balance_ptr: u64) {
    unavailable("account_locked_balance")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn attached_deposit(_balance_ptr: u64) {
    unavailable("attached_deposit")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn prepaid_gas() -> u64 {
    unavailable("prepaid_gas")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn used_gas() -> u64 {
    unavailable("used_gas")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn random_seed(_register_id: u64) {
    unavailable("random_seed")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn sha256(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("sha256")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn keccak256(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("keccak256")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn keccak512(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("keccak512")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn sha3_256(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("sha3_256")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn sha3_384(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("sha3_384")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn sha3_512(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("sha3_512")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn ripemd160(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("ripemd160")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn ecrecover(
    _hash_len: u64,
    _hash_ptr: u64,
    _sig_len: u64,
    _sig_ptr: u64,
    _v: u64,
    _malleability_flag: u64,
    _register_id: u64,
) -> u64 {
    unavailable("ecrecover")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn ed25519_verify(
    _sig_len: u64,
    _sig_ptr: u64,
    _msg_len: u64,
    _msg_ptr: u64,
    _pub_key_len: u64,
    _pub_key_ptr: u64,
) -> u64 {
    unavailable("ed25519_verify")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn p256_verify(
    _sig_len: u64,
    _sig_ptr: u64,
    _msg_len: u64,
    _msg_ptr: u64,
    _pub_key_len: u64,
    _pub_key_ptr: u64,
) -> u64 {
    unavailable("p256_verify")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn ml_dsa_verify(
    _sig_len: u64,
    _sig_ptr: u64,
    _msg_len: u64,
    _msg_ptr: u64,
    _pub_key_len: u64,
    _pub_key_ptr: u64,
) -> u64 {
    unavailable("ml_dsa_verify")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn value_return(_value_len: u64, _value_ptr: u64) {
    unavailable("value_return")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn panic() -> ! {
    unavailable("panic")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn panic_utf8(_len: u64, _ptr: u64) -> ! {
    unavailable("panic_utf8")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn log_utf8(_len: u64, _ptr: u64) {
    unavailable("log_utf8")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn log_utf16(_len: u64, _ptr: u64) {
    unavailable("log_utf16")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn abort(_msg_ptr: u32, _filename_ptr: u32, _line: u32, _col: u32) -> ! {
    unavailable("abort")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_create(
    _account_id_len: u64,
    _account_id_ptr: u64,
    _function_name_len: u64,
    _function_name_ptr: u64,
    _arguments_len: u64,
    _arguments_ptr: u64,
    _amount_ptr: u64,
    _gas: u64,
) -> u64 {
    unavailable("promise_create")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_then(
    _promise_index: u64,
    _account_id_len: u64,
    _account_id_ptr: u64,
    _function_name_len: u64,
    _function_name_ptr: u64,
    _arguments_len: u64,
    _arguments_ptr: u64,
    _amount_ptr: u64,
    _gas: u64,
) -> u64 {
    unavailable("promise_then")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_and(_promise_idx_ptr: u64, _promise_idx_count: u64) -> u64 {
    unavailable("promise_and")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_create(_account_id_len: u64, _account_id_ptr: u64) -> u64 {
    unavailable("promise_batch_create")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_then(
    _promise_index: u64,
    _account_id_len: u64,
    _account_id_ptr: u64,
) -> u64 {
    unavailable("promise_batch_then")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_set_refund_to(
    _promise_index: u64,
    _account_id_len: u64,
    _account_id_ptr: u64,
) {
    unavailable("promise_set_refund_to")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_state_init(
    _promise_index: u64,
    _code_len: u64,
    _code_ptr: u64,
    _amount_ptr: u64,
) -> u64 {
    unavailable("promise_batch_action_state_init")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_state_init_by_account_id(
    _promise_index: u64,
    _account_id_len: u64,
    _account_id_ptr: u64,
    _amount_ptr: u64,
) -> u64 {
    unavailable("promise_batch_action_state_init_by_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn set_state_init_data_entry(
    _promise_index: u64,
    _action_index: u64,
    _key_len: u64,
    _key_ptr: u64,
    _value_len: u64,
    _value_ptr: u64,
) {
    unavailable("set_state_init_data_entry")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_create_account(_promise_index: u64) {
    unavailable("promise_batch_action_create_account")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_deploy_contract(
    _promise_index: u64,
    _code_len: u64,
    _code_ptr: u64,
) {
    unavailable("promise_batch_action_deploy_contract")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_function_call(
    _promise_index: u64,
    _function_name_len: u64,
    _function_name_ptr: u64,
    _arguments_len: u64,
    _arguments_ptr: u64,
    _amount_ptr: u64,
    _gas: u64,
) {
    unavailable("promise_batch_action_function_call")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_function_call_weight(
    _promise_index: u64,
    _function_name_len: u64,
    _function_name_ptr: u64,
    _arguments_len: u64,
    _arguments_ptr: u64,
    _amount_ptr: u64,
    _gas: u64,
    _weight: u64,
) {
    unavailable("promise_batch_action_function_call_weight")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_transfer(_promise_index: u64, _amount_ptr: u64) {
    unavailable("promise_batch_action_transfer")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_stake(
    _promise_index: u64,
    _amount_ptr: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
) {
    unavailable("promise_batch_action_stake")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_add_key_with_full_access(
    _promise_index: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
    _nonce: u64,
) {
    unavailable("promise_batch_action_add_key_with_full_access")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_add_key_with_function_call(
    _promise_index: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
    _nonce: u64,
    _allowance_ptr: u64,
    _receiver_id_len: u64,
    _receiver_id_ptr: u64,
    _function_names_len: u64,
    _function_names_ptr: u64,
) {
    unavailable("promise_batch_action_add_key_with_function_call")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_transfer_to_gas_key(
    _promise_index: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
    _amount_ptr: u64,
) {
    unavailable("promise_batch_action_transfer_to_gas_key")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_add_gas_key_with_full_access(
    _promise_index: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
    _num_nonces: u64,
) {
    unavailable("promise_batch_action_add_gas_key_with_full_access")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_add_gas_key_with_function_call(
    _promise_index: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
    _num_nonces: u64,
    _allowance_ptr: u64,
    _receiver_id_len: u64,
    _receiver_id_ptr: u64,
    _method_names_len: u64,
    _method_names_ptr: u64,
) {
    unavailable("promise_batch_action_add_gas_key_with_function_call")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_delete_key(
    _promise_index: u64,
    _public_key_len: u64,
    _public_key_ptr: u64,
) {
    unavailable("promise_batch_action_delete_key")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_delete_account(
    _promise_index: u64,
    _beneficiary_id_len: u64,
    _beneficiary_id_ptr: u64,
) {
    unavailable("promise_batch_action_delete_account")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_deploy_global_contract(
    _promise_index: u64,
    _code_len: u64,
    _code_ptr: u64,
) {
    unavailable("promise_batch_action_deploy_global_contract")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_deploy_global_contract_by_account_id(
    _promise_index: u64,
    _code_len: u64,
    _code_ptr: u64,
) {
    unavailable("promise_batch_action_deploy_global_contract_by_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_use_global_contract(
    _promise_index: u64,
    _code_hash_len: u64,
    _code_hash_ptr: u64,
) {
    unavailable("promise_batch_action_use_global_contract")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_use_global_contract_by_account_id(
    _promise_index: u64,
    _account_id_len: u64,
    _account_id_ptr: u64,
) {
    unavailable("promise_batch_action_use_global_contract_by_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn universal_state_init_to_account_id(
    _state_init_len: u64,
    _state_init_ptr: u64,
    _register_id: u64,
) {
    unavailable("universal_state_init_to_account_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_batch_action_universal_state_init(
    _promise_index: u64,
    _state_init_len: u64,
    _state_init_ptr: u64,
    _amount_ptr: u64,
) {
    unavailable("promise_batch_action_universal_state_init")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_yield_create(
    _function_name_len: u64,
    _function_name_ptr: u64,
    _arguments_len: u64,
    _arguments_ptr: u64,
    _gas: u64,
    _gas_weight: u64,
    _register_id: u64,
) -> u64 {
    unavailable("promise_yield_create")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_yield_resume(
    _data_id_len: u64,
    _data_id_ptr: u64,
    _payload_len: u64,
    _payload_ptr: u64,
) -> u32 {
    unavailable("promise_yield_resume")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_yield_create_with_id(
    _function_name_len: u64,
    _function_name_ptr: u64,
    _arguments_len: u64,
    _arguments_ptr: u64,
    _amount_ptr: u64,
    _gas: u64,
    _gas_weight: u64,
    _yield_id_len: u64,
    _yield_id_ptr: u64,
) -> u64 {
    unavailable("promise_yield_create_with_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_yield_resume_with_yield_id(
    _yield_id_len: u64,
    _yield_id_ptr: u64,
    _payload_len: u64,
    _payload_ptr: u64,
) -> u32 {
    unavailable("promise_yield_resume_with_yield_id")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_results_count() -> u64 {
    unavailable("promise_results_count")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_result(_result_idx: u64, _register_id: u64) -> u64 {
    unavailable("promise_result")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn promise_return(_promise_id: u64) {
    unavailable("promise_return")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_write(
    _key_len: u64,
    _key_ptr: u64,
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("storage_write")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_read(_key_len: u64, _key_ptr: u64, _register_id: u64) -> u64 {
    unavailable("storage_read")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_remove(_key_len: u64, _key_ptr: u64, _register_id: u64) -> u64 {
    unavailable("storage_remove")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_has_key(_key_len: u64, _key_ptr: u64) -> u64 {
    unavailable("storage_has_key")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_iter_prefix(_prefix_len: u64, _prefix_ptr: u64) -> u64 {
    unavailable("storage_iter_prefix")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_iter_range(
    _start_len: u64,
    _start_ptr: u64,
    _end_len: u64,
    _end_ptr: u64,
) -> u64 {
    unavailable("storage_iter_range")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn storage_iter_next(
    _iterator_id: u64,
    _key_register_id: u64,
    _value_register_id: u64,
) -> u64 {
    unavailable("storage_iter_next")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn validator_stake(_account_id_len: u64, _account_id_ptr: u64, _stake_ptr: u64) {
    unavailable("validator_stake")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn validator_total_stake(_stake_ptr: u64) {
    unavailable("validator_total_stake")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn alt_bn128_g1_multiexp(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("alt_bn128_g1_multiexp")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn alt_bn128_g1_sum(_value_len: u64, _value_ptr: u64, _register_id: u64) {
    unavailable("alt_bn128_g1_sum")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn alt_bn128_pairing_check(_value_len: u64, _value_ptr: u64) -> u64 {
    unavailable("alt_bn128_pairing_check")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_p1_sum(_value_len: u64, _value_ptr: u64, _register_id: u64) -> u64 {
    unavailable("bls12381_p1_sum")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_p2_sum(_value_len: u64, _value_ptr: u64, _register_id: u64) -> u64 {
    unavailable("bls12381_p2_sum")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_g1_multiexp(
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("bls12381_g1_multiexp")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_g2_multiexp(
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("bls12381_g2_multiexp")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_map_fp_to_g1(
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("bls12381_map_fp_to_g1")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_map_fp2_to_g2(
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("bls12381_map_fp2_to_g2")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_pairing_check(_value_len: u64, _value_ptr: u64) -> u64 {
    unavailable("bls12381_pairing_check")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_p1_decompress(
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("bls12381_p1_decompress")
}

#[unsafe(no_mangle)]
extern "C-unwind" fn bls12381_p2_decompress(
    _value_len: u64,
    _value_ptr: u64,
    _register_id: u64,
) -> u64 {
    unavailable("bls12381_p2_decompress")
}
