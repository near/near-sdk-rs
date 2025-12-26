//! Blockchain-specific methods available to the smart contract that allow to interact with NEAR runtime.
//! This is a wrapper around a low-level [`near_sys`](near_sys).
//!
//! Unless you know what you are doing prefer using `env::*`
//! whenever possible.
//!
//! In case of cross-contract calls prefer using higher-level API available
//! through [`crate::Promise`], and [`crate::PromiseOrValue<T>`].

use std::convert::TryFrom;
use std::convert::TryInto;
use std::mem::size_of;
use std::panic as std_panic;

#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
use crate::mock::MockedBlockchain;
use crate::promise::Allowance;
use crate::types::{
    AccountId, AccountIdRef, BlockHeight, Gas, NearToken, PromiseIndex, PromiseResult, PublicKey,
    StorageUsage,
};
use crate::{CryptoHash, GasWeight, PromiseError};

#[cfg(feature = "deterministic-account-ids")]
use crate::{AccountContract, ActionIndex, GlobalContractId};
use near_sys as sys;

const REGISTER_EXPECTED_ERR: &str =
    "Register was expected to have data because we just wrote it into it.";

/// Register used internally for atomic operations. This register is safe to use by the user,
/// since it only needs to be untouched while methods of `Environment` execute, which is guaranteed
/// guest code is not parallel.
const ATOMIC_OP_REGISTER: u64 = u64::MAX - 2;
/// Register used to record evicted values from the storage.
const EVICTED_REGISTER: u64 = u64::MAX - 1;

/// Key used to store the state of the contract.
const STATE_KEY: &[u8] = b"STATE";

/// The minimum length of a valid account ID.
const MIN_ACCOUNT_ID_LEN: u64 = 2;
/// The maximum length of a valid account ID.
const MAX_ACCOUNT_ID_LEN: u64 = 64;

fn expect_register<T>(option: Option<T>) -> T {
    option.unwrap_or_else(|| panic_str(REGISTER_EXPECTED_ERR))
}

/// A simple helper to read blob value coming from host's method.
#[inline]
fn try_method_into_register(method: unsafe extern "C" fn(u64)) -> Option<Vec<u8>> {
    unsafe { method(ATOMIC_OP_REGISTER) };
    read_register(ATOMIC_OP_REGISTER)
}

/// Same as `try_method_into_register` but expects the data.
#[inline]
fn method_into_register(method: unsafe extern "C" fn(u64)) -> Vec<u8> {
    expect_register(try_method_into_register(method))
}

pub(crate) unsafe fn read_register_fixed<const N: usize>(register_id: u64) -> [u8; N] {
    let mut buf = [0; N];
    sys::read_register(register_id, buf.as_mut_ptr() as _);
    buf
}

/// Replaces the current low-level blockchain interface accessible through `env::*` with another
/// low-level blockchain interface with builtin functions of the NEAR runtime. In most cases you
/// want to use `testing_env!` macro to set it.
///
/// ```
/// # let context = near_sdk::test_utils::VMContextBuilder::new().build();
/// # let vm_config = near_sdk::test_vm_config();
/// # let fees_config = near_sdk::RuntimeFeesConfig::test();
/// # let storage = Default::default();
/// # let validators = Default::default();
/// let mocked_blockchain = near_sdk::MockedBlockchain::new(
///           context,
///           vm_config,
///           fees_config,
///           vec![],
///           storage,
///           validators,
///           None,
///       );
/// near_sdk::env::set_blockchain_interface(mocked_blockchain);
/// ```
#[cfg(all(not(target_arch = "wasm32"), feature = "unit-testing"))]
pub fn set_blockchain_interface(blockchain_interface: MockedBlockchain) {
    crate::mock::with_mocked_blockchain(|b| {
        *b = blockchain_interface;
    })
}

/// Implements panic hook that converts `PanicInfo` into a string and provides it through the
/// blockchain interface.
// TODO: replace with std::panic::PanicHookInfo when MSRV becomes >= 1.81.0
#[allow(deprecated)]
fn panic_hook_impl(info: &std_panic::PanicInfo) {
    panic_str(info.to_string().as_str());
}

/// Setups panic hook to expose error info to the blockchain.
pub fn setup_panic_hook() {
    std_panic::set_hook(Box::new(panic_hook_impl));
}

/// Reads the content of the `register_id`. If register is not used returns `None`.
pub fn read_register(register_id: u64) -> Option<Vec<u8>> {
    // Get register length and convert to a usize. The max register size in config is much less
    // than the u32 max so the abort should never be hit, but is there for safety because there
    // would be undefined behaviour during `read_register` if the buffer length is truncated.
    let len: usize = register_len(register_id)?.try_into().unwrap_or_else(|_| abort());

    // Initialize buffer with capacity.
    let mut buffer = Vec::with_capacity(len);

    // Read register into buffer.
    //* SAFETY: This is safe because the buffer is initialized with the exact capacity of the
    //*         register that is being read from.
    unsafe {
        sys::read_register(register_id, buffer.as_mut_ptr() as u64);

        // Set updated length after writing to buffer.
        buffer.set_len(len);
    }
    Some(buffer)
}

/// Returns the size of the register. If register is not used returns `None`.
pub fn register_len(register_id: u64) -> Option<u64> {
    let len = unsafe { sys::register_len(register_id) };
    if len == u64::MAX {
        None
    } else {
        Some(len)
    }
}

macro_rules! maybe_cached {
    ($t:ty: $v:block) => {{
        #[cfg(not(feature = "unit-testing"))]
        {
            static CACHED: ::std::sync::LazyLock<$t> = ::std::sync::LazyLock::new(|| $v);
            CACHED.clone()
        }
        #[cfg(feature = "unit-testing")]
        $v
    }};
}

// ###############
// # Context API #
// ###############
/// The id of the account that owns the current contract.
///
/// # Examples
/// ```
/// use near_sdk::env::current_account_id;
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// assert_eq!(current_account_id(), "alice.near".parse::<AccountId>().unwrap());
/// ```
pub fn current_account_id() -> AccountId {
    maybe_cached!(AccountId: {
        assert_valid_account_id(method_into_register(sys::current_account_id))
    })
}

/// The code of the current contract.
///
/// # Examples
/// ```no_run
/// use near_sdk::env::current_contract_code;
/// use near_sdk::AccountContract;
///
/// assert!(matches!(current_contract_code(), AccountContract::Local(_)));
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn current_contract_code() -> AccountContract {
    maybe_cached!(AccountContract: {
        let mode = unsafe { sys::current_contract_code(ATOMIC_OP_REGISTER) };
        match mode {
            0 => AccountContract::None,
            1 => AccountContract::Local(unsafe { read_register_fixed(ATOMIC_OP_REGISTER) }.into()),
            2 => AccountContract::Global(unsafe { read_register_fixed(ATOMIC_OP_REGISTER) }.into()),
            3 => AccountContract::GlobalByAccount(assert_valid_account_id(method_into_register(
                sys::current_account_id,
            ))),
            _ => panic!("Invalid contract mode"),
        }
    })
}

/// Returns global contract identifier of the contract's code currently being
/// executed. Otherwise, returns `None` if the current contract is not using
/// globally deployed code.
///
/// # Examples
/// ```no_run
/// use near_sdk::env::current_global_contract_id;
/// use near_sdk::GlobalContractId;
///
/// assert!(matches!(current_global_contract_id(), Some(GlobalContractId::CodeHash(_))));
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn current_global_contract_id() -> Option<GlobalContractId> {
    Some(match current_contract_code() {
        AccountContract::Global(hash) => GlobalContractId::CodeHash(hash),
        AccountContract::GlobalByAccount(account_id) => GlobalContractId::AccountId(account_id),
        _ => return None,
    })
}

/// The account id that will receive the refund if the contract panics.
///
/// # Examples
/// ```
/// use near_sdk::env::refund_to_account_id;
///
/// assert_eq!(refund_to_account_id(), "bob.near".parse::<near_sdk::AccountId>().unwrap());
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn refund_to_account_id() -> AccountId {
    maybe_cached!(AccountId: {
        assert_valid_account_id(method_into_register(sys::refund_to_account_id))
    })
}

/// The id of the account that either signed the original transaction or issued the initial
/// cross-contract call.
///
/// # Examples
/// ```
/// use near_sdk::env::signer_account_id;
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// assert_eq!(signer_account_id(), "bob.near".parse::<AccountId>().unwrap());
/// ```
pub fn signer_account_id() -> AccountId {
    maybe_cached!(AccountId: {
        assert_valid_account_id(method_into_register(sys::signer_account_id))
    })
}

/// The public key of the account that did the signing.
///
/// # Examples
/// ```
/// use near_sdk::env::signer_account_pk;
/// use near_sdk::{PublicKey, CurveType};
///
/// let pk = PublicKey::from_parts(near_sdk::CurveType::ED25519, vec![0; 32]).unwrap();
/// assert_eq!(signer_account_pk(), pk);
/// ```
pub fn signer_account_pk() -> PublicKey {
    maybe_cached!(PublicKey: {
        PublicKey::try_from(method_into_register(sys::signer_account_pk))
            .unwrap_or_else(|_| abort())
    })
}

/// The id of the account that was the previous contract in the chain of cross-contract calls.
/// If this is the first contract, it is equal to `signer_account_id`.
///
/// # Examples
/// ```
/// use near_sdk::env::predecessor_account_id;
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// assert_eq!(predecessor_account_id(), "bob.near".parse::<AccountId>().unwrap());
/// ```
pub fn predecessor_account_id() -> AccountId {
    maybe_cached!(AccountId: { assert_valid_account_id(method_into_register(sys::predecessor_account_id)) })
}

/// Helper function to convert and check the account ID from bytes from the runtime.
fn assert_valid_account_id(bytes: Vec<u8>) -> AccountId {
    String::from_utf8(bytes)
        .ok()
        .and_then(|s| AccountId::try_from(s).ok())
        .unwrap_or_else(|| abort())
}

/// The input to the contract call serialized as bytes. If input is not provided returns `None`.
///
/// # Examples
/// ```
/// use near_sdk::env::input;
///
/// assert_eq!(input(), Some(Vec::new()));
/// ```
/// See an example here [here](https://github.com/near-examples/update-migrate-rust/blob/a1a326de73c152831f93fbf6d90932e13a08b89f/self-updates/update/src/update.rs#L19)
pub fn input() -> Option<Vec<u8>> {
    try_method_into_register(sys::input)
}

/// Current block index.
///
/// # Examples
/// ```
/// use near_sdk::env::block_index;
///
/// assert_eq!(block_index(), 0);
/// ```
#[deprecated(since = "4.0.0", note = "Use block_height instead")]
pub fn block_index() -> BlockHeight {
    block_height()
}

/// Returns the height of the block the transaction is being executed in.
///
/// # Examples
/// ```
/// use near_sdk::env::block_height;
///
/// assert_eq!(block_height(), 0);
/// ```
pub fn block_height() -> BlockHeight {
    maybe_cached!(BlockHeight: { unsafe { sys::block_height() } })
}

/// Current block timestamp, i.e, number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
///
/// # Examples
/// ```
/// use near_sdk::env::block_timestamp;
///
/// assert_eq!(block_timestamp(), 0);
/// ```
pub fn block_timestamp() -> u64 {
    maybe_cached!(u64: { unsafe { sys::block_timestamp() } })
}

/// Current block timestamp, i.e, number of non-leap-milliseconds since January 1, 1970 0:00:00 UTC.
///
/// # Examples
/// ```
/// use near_sdk::env::block_timestamp_ms;
///
/// assert_eq!(block_timestamp_ms(), 0);
/// ```
pub fn block_timestamp_ms() -> u64 {
    block_timestamp() / 1_000_000
}

/// Current epoch height.
///
/// # Examples
/// ```
/// use near_sdk::env::epoch_height;
///
/// assert_eq!(epoch_height(), 0);
/// ```
pub fn epoch_height() -> u64 {
    maybe_cached!(u64: { unsafe { sys::epoch_height() } })
}

/// Current total storage usage of this smart contract that this account would be paying for.
///
/// # Examples
/// ```
/// use near_sdk::env::storage_usage;
///
/// assert_eq!(storage_usage(), 307200);
/// ```
pub fn storage_usage() -> StorageUsage {
    unsafe { sys::storage_usage() }
}

// #################
// # Economics API #
// #################
/// The balance attached to the given account. This includes the attached_deposit that was
/// attached to the transaction
///
/// # Examples
/// ```
/// use near_sdk::env::account_balance;
/// use near_sdk::NearToken;
///
/// assert_eq!(account_balance(), NearToken::from_near(100));
/// ```
pub fn account_balance() -> NearToken {
    let mut data = [0u8; size_of::<NearToken>()];
    unsafe { sys::account_balance(data.as_mut_ptr() as u64) };
    NearToken::from_yoctonear(u128::from_le_bytes(data))
}

/// The balance locked for potential validator staking.
///
/// # Examples
/// ```
/// use near_sdk::env::account_locked_balance;
/// use near_sdk::NearToken;
///
/// assert_eq!(account_locked_balance(), NearToken::from_yoctonear(0));
/// ```
pub fn account_locked_balance() -> NearToken {
    let mut data = [0u8; size_of::<NearToken>()];
    unsafe { sys::account_locked_balance(data.as_mut_ptr() as u64) };
    NearToken::from_yoctonear(u128::from_le_bytes(data))
}

/// The balance that was attached to the call that will be immediately deposited before the
/// contract execution starts
///
/// # Examples
/// ```
/// use near_sdk::env::attached_deposit;
/// use near_sdk::NearToken;
///
/// assert_eq!(attached_deposit(), NearToken::from_yoctonear(0));
/// ```
pub fn attached_deposit() -> NearToken {
    maybe_cached!(NearToken: {
        let mut data = [0u8; size_of::<NearToken>()];
        unsafe { sys::attached_deposit(data.as_mut_ptr() as u64) };
        NearToken::from_yoctonear(u128::from_le_bytes(data))
    })
}

/// The amount of gas attached to the call that can be used to pay for the gas fees.
///
/// # Examples
/// ```
/// use near_sdk::env::prepaid_gas;
/// use near_sdk::Gas;
///
/// assert_eq!(prepaid_gas(), Gas::from_tgas(300));
/// ```
pub fn prepaid_gas() -> Gas {
    maybe_cached!(Gas: { Gas::from_gas(unsafe { sys::prepaid_gas() }) })
}

/// The gas that was already burnt during the contract execution (cannot exceed `prepaid_gas`)
///
/// # Examples
/// ```
/// use near_sdk::env::used_gas;
/// use near_sdk::Gas;
///
/// assert_eq!(used_gas(), Gas::from_gas(264768111));
/// ```
pub fn used_gas() -> Gas {
    Gas::from_gas(unsafe { sys::used_gas() })
}

// ############
// # Math API #
// ############

/// Returns the random seed from the current block. This 32 byte hash is based on the VRF value from
/// the block. This value is not modified in any way each time this function is called within the
/// same method/block.
///
/// # Examples
/// ```
/// use near_sdk::env::random_seed;
///
/// assert_eq!(random_seed(), vec![0; 32]);
/// ```
pub fn random_seed() -> Vec<u8> {
    random_seed_array().to_vec()
}

/// Returns the random seed from the current block. This 32 byte hash is based on the VRF value from
/// the block. This value is not modified in any way each time this function is called within the
/// same method/block.
/// Example of usage:
/// ```rust
/// use rand::{Rng, SeedableRng};
/// use rand_chacha::ChaCha20Rng;
/// use near_sdk::near;
/// use near_sdk::env;
/// #[near(contract_state)]
/// struct RngExample {
///    val: i32,
/// }
/// #[near]
/// impl RngExample {
///     pub fn increment(&mut self) {
///         let mut rng = ChaCha20Rng::from_seed(env::random_seed_array());
///         let value = rng.gen_range(0..1011);
///         self.val += value;
///     }
///     pub fn get_value(&mut self) -> i32 {
///         self.val
///     }
/// }
/// ```
///
/// Example of usage with [near-rng](https://lib.rs/crates/near-rng) which allows to decrease size of contract binary:
///
/// ```rust
/// use near_rng::Rng;
/// use near_sdk::near;
/// use near_sdk::env;
/// #[near(contract_state)]
/// struct NearRngExample {
///    val: i32,
/// }
/// #[near]
/// impl NearRngExample {
///     pub fn increment(&mut self) {
///         let mut rng = Rng::new(&env::random_seed());
///         let value = rng.rand_range_i32(0, 20);
///         self.val += value;
///     }
///     pub fn get_value(&mut self) -> i32 {
///         self.val
///     }
/// }
/// ```
/// More info in [documentation](https://docs.near.org/develop/contracts/security/random)
pub fn random_seed_array() -> [u8; 32] {
    maybe_cached!([u8; 32]: {
        //* SAFETY: random_seed syscall will always generate 32 bytes inside of the atomic op register
        //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
        //*         because all bytes are filled. This assumes a valid random_seed implementation.
        unsafe {
            sys::random_seed(ATOMIC_OP_REGISTER);
            read_register_fixed(ATOMIC_OP_REGISTER)
        }
    })
}

/// Hashes the random sequence of bytes using sha256.
///
/// # Examples
/// ```
/// use near_sdk::env::sha256;
/// use hex;
///
/// assert_eq!(
///     sha256(b"The phrase that will be hashed"),
///     hex::decode("7fc38bc74a0d0e592d2b8381839adc2649007d5bca11f92eeddef78681b4e3a3").expect("Decoding failed")
/// );
/// ```
pub fn sha256(value: impl AsRef<[u8]>) -> Vec<u8> {
    sha256_array(value.as_ref()).to_vec()
}

/// Hashes the random sequence of bytes using keccak256.
///
/// # Examples
/// ```
/// use near_sdk::env::keccak256;
/// use hex;
///
/// assert_eq!(
///     keccak256(b"The phrase that will be hashed"),
///     hex::decode("b244af9dd4aada2eda59130bbcff112f29b427d924b654aaeb5a0384fa9afed4")
///         .expect("Decoding failed")
/// );
/// ```
pub fn keccak256(value: impl AsRef<[u8]>) -> Vec<u8> {
    keccak256_array(value.as_ref()).to_vec()
}

/// Hashes the random sequence of bytes using keccak512.
///
/// # Examples
/// ```
/// use near_sdk::env::keccak512;
/// use hex;
///
/// assert_eq!(
///     keccak512(b"The phrase that will be hashed"),
///     hex::decode("29a7df7b889a443fdfbd769adb57ef7e98e6159187b582baba778c06e8b41a75f61367257e8c525a95b3f13ddf432f115d1df128a910c8fc93221db136d92b31")
///         .expect("Decoding failed")
/// );
/// ```
pub fn keccak512(value: impl AsRef<[u8]>) -> Vec<u8> {
    keccak512_array(value.as_ref()).to_vec()
}

/// Hashes the bytes using the SHA-256 hash function. This returns a 32 byte hash.
///
/// # Examples
/// ```
/// use near_sdk::env::sha256_array;
/// use hex;
///
/// assert_eq!(
///     &sha256_array(b"The phrase that will be hashed"),
///     hex::decode("7fc38bc74a0d0e592d2b8381839adc2649007d5bca11f92eeddef78681b4e3a3")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn sha256_array(value: impl AsRef<[u8]>) -> CryptoHash {
    let value = value.as_ref();
    //* SAFETY: sha256 syscall will always generate 32 bytes inside of the atomic op register
    //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
    //*         because all bytes are filled. This assumes a valid sha256 implementation.
    unsafe {
        sys::sha256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
        read_register_fixed(ATOMIC_OP_REGISTER)
    }
}

/// Hashes the bytes using the Keccak-256 hash function. This returns a 32 byte hash.
///
/// # Examples
/// ```
/// use near_sdk::env::keccak256_array;
/// use hex;
///
/// assert_eq!(
///     &keccak256_array(b"The phrase that will be hashed"),
///     hex::decode("b244af9dd4aada2eda59130bbcff112f29b427d924b654aaeb5a0384fa9afed4")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn keccak256_array(value: impl AsRef<[u8]>) -> CryptoHash {
    let value = value.as_ref();
    //* SAFETY: keccak256 syscall will always generate 32 bytes inside of the atomic op register
    //*         so the read will have a sufficient buffer of 32, and can transmute from uninit
    //*         because all bytes are filled. This assumes a valid keccak256 implementation.
    unsafe {
        sys::keccak256(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
        read_register_fixed(ATOMIC_OP_REGISTER)
    }
}

/// Hashes the bytes using the Keccak-512 hash function. This returns a 64 byte hash.
///
/// # Examples
/// ```
/// use near_sdk::env::keccak512_array;
/// use hex;
///
/// assert_eq!(
///     &keccak512_array(b"The phrase that will be hashed"),
///     hex::decode("29a7df7b889a443fdfbd769adb57ef7e98e6159187b582baba778c06e8b41a75f61367257e8c525a95b3f13ddf432f115d1df128a910c8fc93221db136d92b31")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn keccak512_array(value: impl AsRef<[u8]>) -> [u8; 64] {
    let value = value.as_ref();
    //* SAFETY: keccak512 syscall will always generate 64 bytes inside of the atomic op register
    //*         so the read will have a sufficient buffer of 64, and can transmute from uninit
    //*         because all bytes are filled. This assumes a valid keccak512 implementation.
    unsafe {
        sys::keccak512(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
        read_register_fixed(ATOMIC_OP_REGISTER)
    }
}

/// Hashes the bytes using the RIPEMD-160 hash function. This returns a 20 byte hash.
///
/// # Examples
/// ```
/// use near_sdk::env::ripemd160_array;
/// use hex;
///
/// assert_eq!(
///     &ripemd160_array(b"The phrase that will be hashed"),
///     hex::decode("9a48b9195fcb14cfe6051c0a1be7882efcadaed8")
///         .expect("Decoding failed")
///         .as_slice()
/// );
/// ```
pub fn ripemd160_array(value: impl AsRef<[u8]>) -> [u8; 20] {
    let value = value.as_ref();
    //* SAFETY: ripemd160 syscall will always generate 20 bytes inside of the atomic op register
    //*         so the read will have a sufficient buffer of 20, and can transmute from uninit
    //*         because all bytes are filled. This assumes a valid ripemd160 implementation.
    unsafe {
        sys::ripemd160(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
        read_register_fixed(ATOMIC_OP_REGISTER)
    }
}

/// Recovers an ECDSA signer address from a 32-byte message `hash` and a corresponding `signature`
/// along with `v` recovery byte.
///
/// Takes in an additional flag to check for malleability of the signature
/// which is generally only ideal for transactions.
///
/// Returns 64 bytes representing the public key if the recovery was successful.
#[cfg(feature = "unstable")]
pub fn ecrecover(
    hash: &[u8; 32],
    signature: &[u8; 64],
    v: u8,
    malleability_flag: bool,
) -> Option<[u8; 64]> {
    unsafe {
        let return_code = sys::ecrecover(
            hash.len() as _,
            hash.as_ptr() as _,
            signature.len() as _,
            signature.as_ptr() as _,
            v as u64,
            malleability_flag as u64,
            ATOMIC_OP_REGISTER,
        );
        if return_code == 0 {
            None
        } else {
            Some(read_register_fixed(ATOMIC_OP_REGISTER))
        }
    }
}

/// Verifies signature of message using provided ED25519 Public Key
///
/// # Examples
/// ```
/// use near_sdk::env::ed25519_verify;
/// use hex;
///
/// assert_eq!(
///     ed25519_verify(
///         hex::decode("41C44494DAB13009BE73D2CCBD3A49677DDC1F26AD2823CE72833CE4B9603F77CA70A9E179272D92D28E8B2AE7006747C87AB1890362A50347EFF553F5EC4008")
///             .expect("Decoding failed")
///             .as_slice()
///             .try_into()
///             .unwrap(),
///         b"Hello world!",
///         hex::decode("9C16937BF04CCE709FED52344C43634F1E7A05FC29DD41F48844C3588C7FE663")
///             .expect("Decoding failed")
///             .as_slice()
///             .try_into()
///             .unwrap(),
///     ),
///     true
/// );
///
/// assert_eq!(
///     ed25519_verify(
///         hex::decode("41C44494DAB13009BE73D2CCBD3A49677DDC1F26AD2823CE72833CE4B9603F77CA70A9E179272D92D28E8B2AE7006747C87AB1890362A50347EFF553F5EC4008")
///             .expect("Decoding failed")
///             .as_slice()
///             .try_into()
///             .unwrap(),
///         b"Modified message!",
///         hex::decode("9C16937BF04CCE709FED52344C43634F1E7A05FC29DD41F48844C3588C7FE663")
///             .expect("Decoding failed")
///             .as_slice()
///             .try_into()
///             .unwrap(),
///     ),
///     false
/// );
/// ```
pub fn ed25519_verify(
    signature: &[u8; 64],
    message: impl AsRef<[u8]>,
    public_key: &[u8; 32],
) -> bool {
    let message = message.as_ref();
    unsafe {
        sys::ed25519_verify(
            signature.len() as _,
            signature.as_ptr() as _,
            message.len() as _,
            message.as_ptr() as _,
            public_key.len() as _,
            public_key.as_ptr() as _,
        ) == 1
    }
}

/// Compute alt_bn128 g1 multiexp.
///
/// `alt_bn128` is a specific curve from the Barreto-Naehrig(BN) family. It is particularly
/// well-suited for ZK proofs.
///
/// See also: [EIP-196](https://eips.ethereum.org/EIPS/eip-196)
pub fn alt_bn128_g1_multiexp(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::alt_bn128_g1_multiexp(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Compute alt_bn128 g1 sum.
///
/// `alt_bn128` is a specific curve from the Barreto-Naehrig(BN) family. It is particularly
/// well-suited for ZK proofs.
///
/// See also: [EIP-196](https://eips.ethereum.org/EIPS/eip-196)
pub fn alt_bn128_g1_sum(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::alt_bn128_g1_sum(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}
/// Compute pairing check
///
/// `alt_bn128` is a specific curve from the Barreto-Naehrig(BN) family. It is particularly
/// well-suited for ZK proofs.
///
/// See also: [EIP-197](https://eips.ethereum.org/EIPS/eip-197)
pub fn alt_bn128_pairing_check(value: impl AsRef<[u8]>) -> bool {
    let value = value.as_ref();
    unsafe { sys::alt_bn128_pairing_check(value.len() as _, value.as_ptr() as _) == 1 }
}

// #############
// # BLS12-381 #
// #############

/// Compute BLS12-381 G1 sum.
///
/// See also: [IETF draft-irtf-cfrg-pairing-friendly-curves](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-pairing-friendly-curves)
pub fn bls12381_p1_sum(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_p1_sum(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Compute BLS12-381 G2 sum.
pub fn bls12381_p2_sum(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_p2_sum(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Compute BLS12-381 G1 multiexponentiation.
pub fn bls12381_g1_multiexp(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_g1_multiexp(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Compute BLS12-381 G2 multiexponentiation.
pub fn bls12381_g2_multiexp(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_g2_multiexp(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Map an Fp element to a BLS12-381 G1 point.
pub fn bls12381_map_fp_to_g1(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_map_fp_to_g1(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Map an Fp2 element to a BLS12-381 G2 point.
pub fn bls12381_map_fp2_to_g2(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_map_fp2_to_g2(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Perform BLS12-381 pairing check. Returns true if the pairing check passes.
pub fn bls12381_pairing_check(value: impl AsRef<[u8]>) -> bool {
    let value = value.as_ref();
    unsafe { sys::bls12381_pairing_check(value.len() as _, value.as_ptr() as _) == 0 }
}

/// Decompress a BLS12-381 G1 point.
pub fn bls12381_p1_decompress(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_p1_decompress(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

/// Decompress a BLS12-381 G2 point.
pub fn bls12381_p2_decompress(value: impl AsRef<[u8]>) -> Vec<u8> {
    let value = value.as_ref();
    unsafe {
        sys::bls12381_p2_decompress(value.len() as _, value.as_ptr() as _, ATOMIC_OP_REGISTER);
    };
    match read_register(ATOMIC_OP_REGISTER) {
        Some(result) => result,
        None => panic_str(REGISTER_EXPECTED_ERR),
    }
}

// ################
// # Promises API #
// ################
/// Creates a promise that will execute a method on account with given arguments and attaches
/// the given amount and gas.
///
/// # Examples
/// ```
/// use near_sdk::env::promise_create;
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas};
/// use std::str::FromStr;
///
/// let promise = promise_create(
///     "counter.near".parse::<AccountId>().unwrap(),
///     "increment",
///     serde_json::json!({
///         "value": 5
///     }).to_string(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30)
/// );
/// ```
///
/// More info about promises in [NEAR documentation](https://docs.near.org/build/smart-contracts/anatomy/crosscontract#promises)
///
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_create`]
///
/// Example usages of this low-level api are <https://github.com/near/near-sdk-rs/tree/master/examples/factory-contract/low-level/src/lib.rs> and <https://github.com/near/near-sdk-rs/blob/master/examples/cross-contract-calls/low-level/src/lib.rs>
///
pub fn promise_create(
    account_id: impl AsRef<AccountIdRef>,
    function_name: impl AsRef<str>,
    arguments: impl AsRef<[u8]>,
    amount: NearToken,
    gas: Gas,
) -> PromiseIndex {
    let account_id = account_id.as_ref().as_bytes();
    let function_name = function_name.as_ref();
    let arguments = arguments.as_ref();
    unsafe {
        PromiseIndex(sys::promise_create(
            account_id.len() as _,
            account_id.as_ptr() as _,
            function_name.len() as _,
            function_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount.as_yoctonear() as *const u128 as _,
            gas.as_gas(),
        ))
    }
}

/// Attaches the callback (which is a [`near_primitives::action::FunctionCallAction`]) that is executed after promise pointed by `promise_idx` is complete.
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_create, promise_then};
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas};
/// use std::str::FromStr;
///
/// let promise = promise_create(
///     "counter.near".parse::<AccountId>().unwrap(),
///     "increment",
///     serde_json::json!({
///         "value": 5
///     }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30)
/// );
///
/// let chained_promise = promise_then(
///     promise,
///     "greetings.near".parse::<AccountId>().unwrap(),
///     "set_greeting",
///     serde_json::json!({
///         "text": "Hello World"
///     }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(4000000000000),
///     Gas::from_tgas(30)
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_then`]
///
/// Example usages of this low-level api are <https://github.com/near/near-sdk-rs/tree/master/examples/factory-contract/low-level/src/lib.rs> and <https://github.com/near/near-sdk-rs/blob/master/examples/cross-contract-calls/low-level/src/lib.rs>
pub fn promise_then(
    promise_idx: PromiseIndex,
    account_id: impl AsRef<AccountIdRef>,
    function_name: impl AsRef<str>,
    arguments: impl AsRef<[u8]>,
    amount: NearToken,
    gas: Gas,
) -> PromiseIndex {
    let account_id = account_id.as_ref().as_bytes();
    let function_name = function_name.as_ref();
    let arguments = arguments.as_ref();
    unsafe {
        PromiseIndex(sys::promise_then(
            promise_idx.0,
            account_id.len() as _,
            account_id.as_ptr() as _,
            function_name.len() as _,
            function_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount.as_yoctonear() as *const u128 as _,
            gas.as_gas(),
        ))
    }
}

/// Creates a new promise which completes when time all promises passed as arguments complete.
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_create, promise_and};
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas};
/// use std::str::FromStr;
///
/// let promise1 = promise_create(
///     "counter.near".parse::<AccountId>().unwrap(),
///     "increment",
///     serde_json::json!({
///         "value": 5
///     }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30)
/// );
///
/// let promise2 = promise_create(
///     "greetings.near".parse::<AccountId>().unwrap(),
///     "set_greeting",
///     serde_json::json!({
///         "text": "Hello World"
///     }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(4000000000000),
///     Gas::from_tgas(30)
/// );
///
/// let chained_promise = promise_and(&[promise1, promise2]);
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_and`]
pub fn promise_and(promise_indices: &[PromiseIndex]) -> PromiseIndex {
    let data = promise_indices.iter().map(|idx| idx.0.to_le_bytes()).collect::<Vec<_>>();
    unsafe { PromiseIndex(sys::promise_and(data.as_ptr() as _, promise_indices.len() as _)) }
}

/// # Examples
/// ```no_run
///
/// use near_sdk::env;
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// let promise = env::promise_batch_create(
///     &"receiver.near".parse::<AccountId>().unwrap()
/// );
/// ```
/// Create a NEAR promise which will have multiple promise actions inside.
///
/// Example:
/// ```no_run
/// use near_sdk::{env, NearToken, Gas, AccountId};
///
/// let promise_index = env::promise_batch_create(
///     &"example.near".parse::<AccountId>().unwrap()
/// );
///
/// // Adding actions to the promise
/// env::promise_batch_action_transfer(promise_index, NearToken::from_near(10u128)); // Transfer 10 NEAR
/// env::promise_batch_action_function_call(
///     promise_index,
///     "method_name", // Target method
///     b"{}",           // Arguments
///     NearToken::from_near(0), // Attached deposit
///     Gas::from_tgas(5)        // Gas for execution
/// );
/// ```
/// All actions in a batch are executed in the order they were added.
/// Batched actions act as a unit: they execute in the same receipt, and if any fails, then they all get reverted.
/// More information about batching actions can be found in [NEAR documentation](https://docs.near.org/build/smart-contracts/anatomy/actions)
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_create`]
/// See example of usage [here](https://github.com/near/near-sdk-rs/blob/master/examples/factory-contract/low-level/src/lib.rs)
pub fn promise_batch_create(account_id: impl AsRef<AccountIdRef>) -> PromiseIndex {
    let account_id = account_id.as_ref().as_str();
    unsafe {
        PromiseIndex(sys::promise_batch_create(account_id.len() as _, account_id.as_ptr() as _))
    }
}

/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_then, promise_create};
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas};
/// use std::str::FromStr;
///
/// let promise = promise_create(
///     "counter.near".parse::<AccountId>().unwrap(),
///     "increment",
///     serde_json::json!({
///         "value": 5
///     }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30)
/// );
///
/// let new_promise = promise_batch_then(
///     promise,
///     "receiver.near".parse::<AccountId>().unwrap()
/// );
/// ```
/// Attach a callback NEAR promise to a batch of NEAR promise actions.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_then`]
pub fn promise_batch_then(
    promise_index: PromiseIndex,
    account_id: impl AsRef<AccountIdRef>,
) -> PromiseIndex {
    let account_id = account_id.as_ref().as_str();
    unsafe {
        PromiseIndex(sys::promise_batch_then(
            promise_index.0,
            account_id.len() as _,
            account_id.as_ptr() as _,
        ))
    }
}

/// Set the account id that will receive the refund if the promise panics.
/// Uses low-level [`crate::sys::promise_set_refund_to`]
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_set_refund_to, promise_create};
/// use near_sdk::{AccountId, Gas, NearToken};
/// use std::str::FromStr;
///
/// let promise = promise_create(
///     "account.near".parse::<AccountId>().unwrap(),
///     "method",
///     [],
///     NearToken::from_millinear(1),
///     Gas::from_tgas(1),
/// );
/// promise_set_refund_to(promise, "refund.near".parse::<AccountId>().unwrap());
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn promise_set_refund_to(promise_index: PromiseIndex, account_id: impl AsRef<AccountIdRef>) {
    let account_id = account_id.as_ref().as_str();
    unsafe {
        sys::promise_set_refund_to(promise_index.0, account_id.len() as _, account_id.as_ptr() as _)
    }
}

/// Appends `DeterministicStateInit` action to the batch of actions for the given promise
/// pointed by `promise_index`.
/// Uses low-level [`crate::sys::promise_batch_action_state_init`]
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_state_init, promise_create};
/// use near_sdk::{AccountId, CryptoHash, Gas, NearToken};
///
/// let promise_idx = promise_create(
///     "account.near".parse::<AccountId>().unwrap(),
///     "method",
///     [],
///     NearToken::from_millinear(1),
///     Gas::from_tgas(1),
/// );
/// promise_batch_action_state_init(promise, [0; 32], NearToken::from_millinear(1));
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn promise_batch_action_state_init(
    promise_index: PromiseIndex,
    code: CryptoHash,
    amount: NearToken,
) -> ActionIndex {
    unsafe {
        sys::promise_batch_action_state_init(
            promise_index.0,
            code.len() as _,
            code.as_ptr() as _,
            &amount.as_yoctonear() as *const u128 as _,
        )
    }
}

/// Appends `DeterministicStateInit` action to the batch of actions for the given promise
/// pointed by `promise_index`.
/// Uses low-level [`crate::sys::promise_batch_action_state_init_by_account_id`]
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_state_init_by_account_id, promise_create};
/// use near_sdk::{AccountId, Gas, NearToken};
///
/// let promise = promise_create(
///     "account.near".parse::<AccountId>().unwrap(),
///     "method",
///     [],
///     NearToken::from_millinear(1),
///     Gas::from_tgas(1),
/// );
/// promise_batch_action_state_init_by_account_id(promise, "account.near".parse::<AccountId>().unwrap(), NearToken::from_millinear(1));
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn promise_batch_action_state_init_by_account_id(
    promise_index: PromiseIndex,
    account_id: impl AsRef<AccountIdRef>,
    amount: NearToken,
) -> ActionIndex {
    let account_id = account_id.as_ref().as_bytes();
    unsafe {
        sys::promise_batch_action_state_init_by_account_id(
            promise_index.0,
            account_id.len() as _,
            account_id.as_ptr() as _,
            &amount.as_yoctonear() as *const u128 as _,
        )
    }
}

/// Appends a data entry to an existing `DeterministicStateInit` action.
/// Uses low-level [`crate::sys::set_state_init_data_entry`]
///
/// # Examples
/// ```
/// use near_sdk::env::{set_state_init_data_entry, promise_batch_action_state_init_by_account_id, promise_create};
/// use near_sdk::{AccountId, Gas, NearToken};
///
/// let promise = promise_create(
///     "account.near".parse::<AccountId>().unwrap(),
///     "method",
///     [],
///     NearToken::from_millinear(1),
///     Gas::from_tgas(1),
/// );
/// let action_index = promise_batch_action_state_init_by_account_id(promise, "account.near".parse::<AccountId>().unwrap(), NearToken::from_millinear(1));
/// set_state_init_data_entry(promise, action_index, b"key", b"value");
/// ```
#[cfg(feature = "deterministic-account-ids")]
pub fn set_state_init_data_entry(
    promise_index: PromiseIndex,
    action_index: ActionIndex,
    key: impl AsRef<[u8]>,
    value: impl AsRef<[u8]>,
) {
    let key = key.as_ref();
    let value = value.as_ref();
    unsafe {
        sys::set_state_init_data_entry(
            promise_index.0,
            action_index,
            key.len() as _,
            key.as_ptr() as _,
            value.len() as _,
            value.as_ptr() as _,
        )
    }
}
/// Attach a create account promise action to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_create_account, promise_batch_create};
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("new_account.near").unwrap()
/// );
///
/// promise_batch_action_create_account(promise);
/// ```
///
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_create_account`]
/// See example of usage [here](https://github.com/near/near-sdk-rs/blob/master/examples/factory-contract/low-level/src/lib.rs)
pub fn promise_batch_action_create_account(promise_index: PromiseIndex) {
    unsafe { sys::promise_batch_action_create_account(promise_index.0) }
}

/// Attach a deploy contract promise action to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_deploy_contract, promise_batch_create};
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("contract.near").unwrap()
/// );
///
/// let code = [0; 1487];
/// promise_batch_action_deploy_contract(promise, &code);
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_deploy_contract`]
/// See example of usage [here](https://github.com/near/near-sdk-rs/blob/master/examples/factory-contract/low-level/src/lib.rs)
pub fn promise_batch_action_deploy_contract(promise_index: PromiseIndex, code: impl AsRef<[u8]>) {
    let code = code.as_ref();
    unsafe {
        sys::promise_batch_action_deploy_contract(
            promise_index.0,
            code.len() as _,
            code.as_ptr() as _,
        )
    }
}

/// Attach a function call promise action to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_function_call, promise_batch_create};
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("counter.near").unwrap()
/// );
///
/// promise_batch_action_function_call(
///     promise,
///     "increase",
///     serde_json::json!({ "value": 5 }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30)
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_function_call`]
pub fn promise_batch_action_function_call(
    promise_index: PromiseIndex,
    function_name: impl AsRef<str>,
    arguments: impl AsRef<[u8]>,
    amount: NearToken,
    gas: Gas,
) {
    let function_name = function_name.as_ref();
    let arguments = arguments.as_ref();
    unsafe {
        sys::promise_batch_action_function_call(
            promise_index.0,
            function_name.len() as _,
            function_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount.as_yoctonear() as *const u128 as _,
            gas.as_gas(),
        )
    }
}

/// Attach a function call with specific gas weight promise action to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_function_call_weight, promise_batch_create};
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas, GasWeight};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("counter.near").unwrap()
/// );
///
/// promise_batch_action_function_call_weight(
///     promise,
///     "increase",
///     serde_json::json!({ "value": 5 }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30),
///     GasWeight(1)
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_function_call_weight`]
pub fn promise_batch_action_function_call_weight(
    promise_index: PromiseIndex,
    function_name: impl AsRef<str>,
    arguments: impl AsRef<[u8]>,
    amount: NearToken,
    gas: Gas,
    weight: GasWeight,
) {
    let function_name = function_name.as_ref();
    let arguments = arguments.as_ref();
    unsafe {
        sys::promise_batch_action_function_call_weight(
            promise_index.0,
            function_name.len() as _,
            function_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            &amount.as_yoctonear() as *const u128 as _,
            gas.as_gas(),
            weight.0,
        )
    }
}

/// Attach a transfer promise action to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_transfer, promise_batch_create};
/// use near_sdk::{NearToken, AccountId};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// promise_batch_action_transfer(
///     promise,
///     NearToken::from_near(1),
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_transfer`]
/// See example of usage [here](https://github.com/near/near-sdk-rs/blob/master/examples/factory-contract/low-level/src/lib.rs)
pub fn promise_batch_action_transfer(promise_index: PromiseIndex, amount: NearToken) {
    unsafe {
        sys::promise_batch_action_transfer(
            promise_index.0,
            &amount.as_yoctonear() as *const u128 as _,
        )
    }
}

/// Attach a stake promise action to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_stake, promise_batch_create};
/// use near_sdk::{NearToken, PublicKey, AccountId};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// let pk: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj".parse().unwrap();
/// promise_batch_action_stake(
///     promise,
///     NearToken::from_near(1),
///     &pk
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_stake`]
pub fn promise_batch_action_stake(
    promise_index: PromiseIndex,
    amount: NearToken,
    public_key: &PublicKey,
) {
    unsafe {
        sys::promise_batch_action_stake(
            promise_index.0,
            &amount.as_yoctonear() as *const u128 as _,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
        )
    }
}

/// Attach promise action that adds a full access key to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_add_key_with_full_access, promise_batch_create};
/// use near_sdk::{PublicKey, AccountId};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// let pk: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj".parse().unwrap();
/// let nonce = 55;
/// promise_batch_action_add_key_with_full_access(
///     promise,
///     &pk,
///     nonce
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_add_key_with_full_access`]
/// See example of usage [here](https://github.com/near/near-sdk-rs/blob/master/examples/factory-contract/low-level/src/lib.rs)
pub fn promise_batch_action_add_key_with_full_access(
    promise_index: PromiseIndex,
    public_key: &PublicKey,
    nonce: u64,
) {
    unsafe {
        sys::promise_batch_action_add_key_with_full_access(
            promise_index.0,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
            nonce,
        )
    }
}

/// This is a short lived function while we migrate between the Balance and the allowance type
///
/// More info about batching [here](crate::env::promise_batch_create)
pub(crate) fn migrate_to_allowance(allowance: NearToken) -> Allowance {
    Allowance::limited(allowance).unwrap_or(Allowance::Unlimited)
}

/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_add_key_with_function_call, promise_batch_create};
/// use near_sdk::{PublicKey, AccountId, NearToken};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// let pk: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj".parse().unwrap();
/// let nonce = 55;
/// promise_batch_action_add_key_with_function_call(
///     promise,
///     &pk,
///     nonce,
///     NearToken::from_near(1),
///     &AccountId::from_str("counter.near").unwrap(),
///     "increase,decrease"
/// );
/// ```
#[deprecated(since = "5.0.0", note = "Use add_access_key_allowance instead")]
pub fn promise_batch_action_add_key_with_function_call(
    promise_index: PromiseIndex,
    public_key: &PublicKey,
    nonce: u64,
    allowance: NearToken,
    receiver_id: impl AsRef<AccountIdRef>,
    function_names: impl AsRef<str>,
) {
    let allowance = migrate_to_allowance(allowance);
    promise_batch_action_add_key_allowance_with_function_call(
        promise_index,
        public_key,
        nonce,
        allowance,
        receiver_id,
        function_names,
    )
}

/// Attach promise action that adds a key with function call with specifi allowance to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// Unlimited allowance
/// ```
/// use near_sdk::env::{promise_batch_action_add_key_allowance_with_function_call, promise_batch_create};
/// use near_sdk::{PublicKey, AccountId, Allowance};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// let pk: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj".parse().unwrap();
/// let nonce = 55;
/// promise_batch_action_add_key_allowance_with_function_call(
///     promise,
///     &pk,
///     nonce,
///     Allowance::unlimited(),
///     &AccountId::from_str("counter.near").unwrap(),
///     "increase,decrease"
/// );
/// ```
///
/// Limited allowance (1 NEAR)
/// ```
/// use near_sdk::env::{promise_batch_action_add_key_allowance_with_function_call, promise_batch_create};
/// use near_sdk::{PublicKey, AccountId, Allowance, NearToken};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// let pk: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj".parse().unwrap();
/// let nonce = 55;
/// promise_batch_action_add_key_allowance_with_function_call(
///     promise,
///     &pk,
///     nonce,
///     Allowance::limited(NearToken::from_near(1)).unwrap(),
///     &AccountId::from_str("counter.near").unwrap(),
///     "increase,decrease"
/// );
/// ```
pub fn promise_batch_action_add_key_allowance_with_function_call(
    promise_index: PromiseIndex,
    public_key: &PublicKey,
    nonce: u64,
    allowance: Allowance,
    receiver_id: impl AsRef<AccountIdRef>,
    function_names: impl AsRef<str>,
) {
    let receiver_id = receiver_id.as_ref().as_str();
    let function_names = function_names.as_ref();
    let allowance = match allowance {
        Allowance::Limited(x) => x.get(),
        Allowance::Unlimited => 0,
    };
    unsafe {
        sys::promise_batch_action_add_key_with_function_call(
            promise_index.0,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
            nonce,
            &allowance as *const u128 as _,
            receiver_id.len() as _,
            receiver_id.as_ptr() as _,
            function_names.len() as _,
            function_names.as_ptr() as _,
        )
    }
}

/// Attach promise action that deletes the key to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_delete_key, promise_batch_create};
/// use near_sdk::{PublicKey, AccountId};
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// let pk: PublicKey = "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj".parse().unwrap();
/// promise_batch_action_delete_key(
///     promise,
///     &pk
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_delete_key`]
pub fn promise_batch_action_delete_key(promise_index: PromiseIndex, public_key: &PublicKey) {
    unsafe {
        sys::promise_batch_action_delete_key(
            promise_index.0,
            public_key.as_bytes().len() as _,
            public_key.as_bytes().as_ptr() as _,
        )
    }
}

/// Attach promise action that deletes the account to the NEAR promise index with the provided promise index.
///
/// More info about batching [here](crate::env::promise_batch_create)
/// # Examples
/// ```
/// use near_sdk::env::{promise_batch_action_delete_account, promise_batch_create};
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// let promise = promise_batch_create(
///     &AccountId::from_str("receiver.near").unwrap()
/// );
///
/// promise_batch_action_delete_account(
///     promise,
///     &AccountId::from_str("beneficiary.near").unwrap()
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_batch_action_delete_account`]
pub fn promise_batch_action_delete_account(
    promise_index: PromiseIndex,
    beneficiary_id: impl AsRef<AccountIdRef>,
) {
    let beneficiary_id = beneficiary_id.as_ref().as_str();
    unsafe {
        sys::promise_batch_action_delete_account(
            promise_index.0,
            beneficiary_id.len() as _,
            beneficiary_id.as_ptr() as _,
        )
    }
}

#[cfg(feature = "global-contracts")]
/// Deploys a global contract using the provided contract code.
///
/// # Arguments
/// * `promise_index` - Promise batch index
/// * `code` - Contract bytecode to deploy as a global contract
///
/// # Examples
/// ```no_run
/// use near_sdk::{env, AccountId, PromiseIndex};
///
/// let promise = env::promise_batch_create("alice.near".parse::<AccountId>().unwrap());
/// let code = vec![0u8; 100]; // Contract bytecode
/// env::promise_batch_action_deploy_global_contract(promise, code);
/// ```
pub fn promise_batch_action_deploy_global_contract(
    promise_index: PromiseIndex,
    code: impl AsRef<[u8]>,
) {
    let code = code.as_ref();
    unsafe {
        sys::promise_batch_action_deploy_global_contract(
            promise_index.0,
            code.len() as _,
            code.as_ptr() as _,
        )
    }
}

#[cfg(feature = "global-contracts")]
/// Deploys a global contract by referencing another account's deployed code.
///
/// # Arguments
/// * `promise_index` - Promise batch index
/// * `code` - Contract bytecode to deploy as a global contract
///
/// # Examples
/// ```no_run
/// use near_sdk::{env, AccountId, PromiseIndex};
///
/// let promise = env::promise_batch_create(&"alice.near".parse::<AccountId>().unwrap());
/// let code = vec![0u8; 100]; // Contract bytecode
/// env::promise_batch_action_deploy_global_contract_by_account_id(promise, &code);
/// ```
pub fn promise_batch_action_deploy_global_contract_by_account_id(
    promise_index: PromiseIndex,
    code: impl AsRef<[u8]>,
) {
    let code = code.as_ref();
    unsafe {
        sys::promise_batch_action_deploy_global_contract_by_account_id(
            promise_index.0,
            code.len() as _,
            code.as_ptr() as _,
        )
    }
}

#[cfg(feature = "global-contracts")]
/// Uses an existing global contract by code hash.
///
/// # Arguments
/// * `promise_index` - Promise batch index
/// * `code_hash` - Hash of the global contract code to use
///
/// # Examples
/// ```no_run
/// use near_sdk::{env, AccountId, PromiseIndex};
///
/// let promise = env::promise_batch_create(&"alice.near".parse::<AccountId>().unwrap());
/// let code_hash = [0u8; 32]; // 32-byte hash (CryptoHash)
/// env::promise_batch_action_use_global_contract(promise, &code_hash);
/// ```
pub fn promise_batch_action_use_global_contract(
    promise_index: PromiseIndex,
    code_hash: &CryptoHash,
) {
    unsafe {
        sys::promise_batch_action_use_global_contract(
            promise_index.0,
            code_hash.len() as _,
            code_hash.as_ptr() as _,
        )
    }
}

#[cfg(feature = "global-contracts")]
/// Uses an existing global contract by referencing the account that deployed it.
///
/// # Arguments
/// * `promise_index` - Promise batch index
/// * `account_id` - Account ID that deployed the global contract
///
/// # Examples
/// ```no_run
/// use near_sdk::{env, PromiseIndex, AccountId};
/// use std::str::FromStr;
///
/// let promise = env::promise_batch_create(&"alice.near".parse::<AccountId>().unwrap());
/// env::promise_batch_action_use_global_contract_by_account_id(
///     promise,
///     AccountId::from_str("deployer.near").unwrap()
/// );
/// ```
pub fn promise_batch_action_use_global_contract_by_account_id(
    promise_index: PromiseIndex,
    account_id: impl AsRef<AccountIdRef>,
) {
    let account_id = account_id.as_ref().as_bytes();
    unsafe {
        sys::promise_batch_action_use_global_contract_by_account_id(
            promise_index.0,
            account_id.len() as _,
            account_id.as_ptr() as _,
        )
    }
}

/// If the current function is invoked by a callback we can access the execution results of the
/// promises that caused the callback. This function returns the number of complete and
/// incomplete callbacks.
///
/// # Examples
/// ```
/// use near_sdk::env::promise_results_count;
///
/// assert_eq!(promise_results_count(), 0);
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_results_count`]
///
/// See example of usage [here](https://github.com/near/near-sdk-rs/blob/master/examples/cross-contract-calls/low-level/src/lib.rs)
pub fn promise_results_count() -> u64 {
    maybe_cached!(u64: { unsafe { sys::promise_results_count() } })
}
/// If the current function is invoked by a callback we can access the execution results of the
/// promises that caused the callback.
///
/// # Examples
/// ```no_run
/// use near_sdk::env::{promise_result, promise_results_count, log_str};
/// use near_sdk::PromiseResult;
///
/// assert!(promise_results_count() > 0);
///
/// // The promise_index will be in the range [0, n)
/// // where n is the number of promises triggering this callback,
/// // retrieved from promise_results_count()
/// let promise_index = 0;
/// let result = promise_result(promise_index);
///
/// match result {
///     PromiseResult::Successful(data) => {
///         log_str(format!("Result as Vec<u8>: {:?}", data).as_str());
///     }
///     PromiseResult::Failed => {
///         log_str("Promise failed!");
///     }
/// };
/// ```
///
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_result`]
///
/// Example usages:
/// - [near-contract-standards/src/fungible_token](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/near-contract-standards/src/fungible_token/core_impl.rs#L178)
/// - [near-contract-standards/src/non_fungible_token](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/near-contract-standards/src/non_fungible_token/core/core_impl.rs#L433)
/// - [examples/factory-contract/low-level](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/examples/factory-contract/low-level/src/lib.rs#L61)
/// - [examples/cross-contract-calls/low-level](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/examples/cross-contract-calls/low-level/src/lib.rs#L46)
pub fn promise_result(result_idx: u64) -> PromiseResult {
    match promise_result_internal(result_idx) {
        Ok(()) => {
            let data = expect_register(read_register(ATOMIC_OP_REGISTER));
            PromiseResult::Successful(data)
        }
        Err(PromiseError::Failed) => PromiseResult::Failed,
    }
}

pub(crate) fn promise_result_internal(result_idx: u64) -> Result<(), PromiseError> {
    match unsafe { sys::promise_result(result_idx, ATOMIC_OP_REGISTER) } {
        1 => Ok(()),
        2 => Err(PromiseError::Failed),
        _ => abort(),
    }
}

/// Consider the execution result of promise under `promise_idx` as execution result of this
/// function.
///
/// # Examples
/// ```
/// use near_sdk::env::{promise_create, promise_return};
/// use near_sdk::serde_json;
/// use near_sdk::{AccountId, NearToken, Gas};
/// use std::str::FromStr;
///
/// let promise = promise_create(
///     AccountId::from_str("counter.near").unwrap(),
///     "increment",
///     serde_json::json!({
///         "value": 5
///     }).to_string().into_bytes().as_slice(),
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(30)
/// );
///
/// promise_return(promise);
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_return`]
///
/// Example usages: [one](https://github.com/near/near-sdk-rs/tree/master/examples/cross-contract-calls/low-level/src/lib.rs), [two](https://github.com/near/near-sdk-rs/tree/master/examples/factory-contract/low-level/src/lib.rs)
pub fn promise_return(promise_idx: PromiseIndex) {
    unsafe { sys::promise_return(promise_idx.0) }
}

/// Creates a promise that will execute a method on the current account with given arguments.
/// Writes a resumption token (data id) to the specified register. The callback method will execute
/// after promise_yield_resume is called with the data id OR enough blocks have passed. The timeout
/// length is specified as a protocol-level parameter yield_timeout_length_in_blocks = 200.
///
/// The callback method will execute with a single promise input. Input will either be a payload
/// provided by the user when calling promise_yield_resume, or a PromiseError in case of timeout.
///
/// Resumption tokens are specific to the local account; promise_yield_resume must be called from
/// a method of the same contract.
///
/// # Examples
/// ```no_run
/// use near_sdk::env::{promise_yield_create, promise_yield_resume, read_register};
/// use near_sdk::serde_json;
/// use near_sdk::{Gas, GasWeight, CryptoHash};
///
/// let DATA_ID_REGISTER = 0;
/// // Create yield promise
/// let promise = promise_yield_create(
///     "increment",
///     // passed as arguments
///     serde_json::json!({
///         "value": 5
///     }).to_string().into_bytes().as_slice(),
///     Gas::from_tgas(10),
///     GasWeight(0),
///     DATA_ID_REGISTER
/// );
///
/// // Retrieve `data_id` for further resume
/// let data_id: CryptoHash = read_register(DATA_ID_REGISTER)
///     .expect("read_register failed")
///     .try_into()
///     .expect("conversion to CryptoHash failed");
///
/// // Resume execution using previously retrieved `data_id`
/// promise_yield_resume(
///     &data_id,
///     // passed as callback_result
///     serde_json::json!({
///         "key": "value",
///         "description": "some text"
///     }).to_string().into_bytes().as_slice()
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_yield_create`]
/// See example of usage [here](https://github.com/near/mpc/blob/79ec50759146221e7ad8bb04520f13333b75ca07/chain-signatures/contract/src/lib.rs#L689) and [here](https://github.com/near/near-sdk-rs/blob/master/examples/mpc-contract/src/lib.rs#L45)
pub fn promise_yield_create(
    function_name: impl AsRef<str>,
    arguments: impl AsRef<[u8]>,
    gas: Gas,
    weight: GasWeight,
    register_id: u64,
) -> PromiseIndex {
    let function_name = function_name.as_ref();
    let arguments = arguments.as_ref();
    unsafe {
        PromiseIndex(sys::promise_yield_create(
            function_name.len() as _,
            function_name.as_ptr() as _,
            arguments.len() as _,
            arguments.as_ptr() as _,
            gas.as_gas(),
            weight.0,
            register_id as _,
        ))
    }
}

/// Helper function that creates a yield promise and returns both the promise index and the yield ID.
///
/// This is a convenience wrapper around [`promise_yield_create`] that automatically reads the
/// yield ID from the register and returns it as a [`crate::YieldId`].
pub fn promise_yield_create_id(
    function_name: impl AsRef<str>,
    arguments: impl AsRef<[u8]>,
    gas: Gas,
    weight: GasWeight,
) -> (PromiseIndex, crate::YieldId) {
    let promise_index =
        promise_yield_create(function_name, arguments, gas, weight, ATOMIC_OP_REGISTER);
    // SAFETY: promise_yield_create writes a 32-byte yield ID to the register
    let yield_id = crate::YieldId(unsafe { read_register_fixed(ATOMIC_OP_REGISTER) });
    (promise_index, yield_id)
}

/// Accepts a resumption token `data_id` created by promise_yield_create on the local account.
/// `data` is a payload to be passed to the callback method as a promise input. Returns false if
/// no promise yield with the specified `data_id` is found. Returns true otherwise, guaranteeing
/// that the callback method will be executed with a user-provided payload.
///
/// If promise_yield_resume is called multiple times with the same `data_id`, it is possible to get
/// back multiple 'true' results. The payload from the first successful call is passed to the
/// callback.
///
/// # Examples
/// ```no_run
/// use near_sdk::env::{promise_yield_create, promise_yield_resume, read_register};
/// use near_sdk::serde_json;
/// use near_sdk::{Gas, GasWeight, CryptoHash};
///
/// let DATA_ID_REGISTER = 0;
/// // Create yield promise
/// let promise = promise_yield_create(
///     "increment",
///     // passed as arguments
///     serde_json::json!({
///         "value": 5
///     }).to_string().into_bytes().as_slice(),
///     Gas::from_tgas(10),
///     GasWeight(0),
///     DATA_ID_REGISTER
/// );
///
/// // Retrieve `data_id` for further resume
/// let data_id: CryptoHash = read_register(DATA_ID_REGISTER)
///     .expect("read_register failed")
///     .try_into()
///     .expect("conversion to CryptoHash failed");
///
/// // Resume execution using previously retrieved `data_id`
/// promise_yield_resume(
///     &data_id,
///     // passed as callback_result
///     serde_json::json!({
///         "key": "value",
///         "description": "some text"
///     }).to_string().into_bytes().as_slice()
/// );
/// ```
/// More low-level info here: [`near_vm_runner::logic::VMLogic::promise_yield_resume`]
/// See example of usage [here](https://github.com/near/mpc/blob/79ec50759146221e7ad8bb04520f13333b75ca07/chain-signatures/contract/src/lib.rs#L288) and [here](https://github.com/near/near-sdk-rs/blob/master/examples/mpc-contract/src/lib.rs#L84)
pub fn promise_yield_resume(data_id: &CryptoHash, data: impl AsRef<[u8]>) -> bool {
    let data = data.as_ref();
    unsafe {
        sys::promise_yield_resume(
            data_id.len() as _,
            data_id.as_ptr() as _,
            data.len() as _,
            data.as_ptr() as _,
        ) != 0
    }
}

// ###############
// # Validator API #
// ###############

/// For a given account return its current stake. If the account is not a validator, returns 0.
///
/// # Examples
/// ```
/// use near_sdk::env::validator_stake;
/// use near_sdk::{AccountId, NearToken};
/// use std::str::FromStr;
///
/// assert_eq!(
///     validator_stake(&AccountId::from_str("bob.near").unwrap()),
///     NearToken::from_yoctonear(0)
/// );
/// ```
pub fn validator_stake(account_id: impl AsRef<AccountIdRef>) -> NearToken {
    let account_id = account_id.as_ref().as_str();
    let mut data = [0u8; size_of::<NearToken>()];
    unsafe {
        sys::validator_stake(
            account_id.len() as _,
            account_id.as_ptr() as _,
            data.as_mut_ptr() as u64,
        )
    };
    NearToken::from_yoctonear(u128::from_le_bytes(data))
}

/// Returns the total stake of validators in the current epoch.
///
/// # Examples
/// ```
/// use near_sdk::env::validator_total_stake;
/// use near_sdk::NearToken;
///
/// assert_eq!(
///     validator_total_stake(),
///     NearToken::from_yoctonear(0)
/// );
/// ```
pub fn validator_total_stake() -> NearToken {
    let mut data = [0u8; size_of::<NearToken>()];
    unsafe { sys::validator_total_stake(data.as_mut_ptr() as u64) };
    NearToken::from_yoctonear(u128::from_le_bytes(data))
}

// #####################
// # Miscellaneous API #
// #####################
/// Sets the blob of data as the return value of the contract.
///
/// # Examples
/// ```
/// use near_sdk::env::value_return;
///
/// value_return(b"String data");
/// ```
/// ```
/// use near_sdk::env::value_return;
/// use near_sdk::serde_json;
///
/// value_return(
///     serde_json::json!({
///         "account": "test.near",
///         "value": 5
///     }).to_string().into_bytes().as_slice()
/// );
/// ```
/// Example of usage [here](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/examples/cross-contract-calls/low-level/src/lib.rs#L18)
pub fn value_return(value: impl AsRef<[u8]>) {
    let value = value.as_ref();
    unsafe { sys::value_return(value.len() as _, value.as_ptr() as _) }
}
/// Terminates the execution of the program with the UTF-8 encoded message.
/// [`panic_str`] should be used as the bytes are required to be UTF-8
///
/// # Examples
/// ```should_panic
/// use near_sdk::env::panic;
///
/// panic(b"Unexpected error");
/// ```
#[deprecated(since = "4.0.0", note = "Use env::panic_str to panic with a message.")]
pub fn panic(message: impl AsRef<[u8]>) -> ! {
    let message = message.as_ref();
    unsafe { sys::panic_utf8(message.len() as _, message.as_ptr() as _) }
}

/// Terminates the execution of the program with the UTF-8 encoded message.
///
/// # Examples
/// ```should_panic
/// use near_sdk::env::panic_str;
///
/// panic_str("Unexpected error");
/// ```
/// ```should_panic
/// use near_sdk::env::panic_str;
/// use near_sdk::AccountId;
/// use std::str::FromStr;
///
/// let account = AccountId::from_str("bob.near").unwrap();
/// panic_str(format!("Unexpected error happened for account {}", account).as_str());
/// ```
pub fn panic_str(message: impl AsRef<str>) -> ! {
    let message = message.as_ref();
    unsafe { sys::panic_utf8(message.len() as _, message.as_ptr() as _) }
}

/// Aborts the current contract execution without a custom message.
/// To include a message, use [`panic_str`].
///
/// # Examples
/// ```should_panic
/// use near_sdk::env::abort;
///
/// abort();
/// ```
pub fn abort() -> ! {
    // Use wasm32 unreachable call to avoid including the `panic` external function in Wasm.
    #[cfg(target_arch = "wasm32")]
    //* This was stabilized recently (~ >1.51), so ignore warnings but don't enforce higher msrv
    #[allow(unused_unsafe)]
    unsafe {
        core::arch::wasm32::unreachable()
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe {
        sys::panic()
    }
}

/// Logs the string message message. This message is stored on chain.
///
/// # Examples
/// ```
/// use near_sdk::env::log_str;
///
/// log_str("Some text");
/// ```
/// ```
/// use near_sdk::env::log_str;
///
/// let number = 5;
/// log_str(format!("Number: {}", number).as_str());
/// ```
/// Example of usage [here](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/near-contract-standards/src/event.rs#L29)
pub fn log_str(message: impl AsRef<str>) {
    let message = message.as_ref();
    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    eprintln!("{message}");

    unsafe { sys::log_utf8(message.len() as _, message.as_ptr() as _) }
}

/// Log the UTF-8 encodable message.
///
/// # Examples
/// ```
/// use near_sdk::env::log;
///
/// log(b"Text");
/// ```
#[deprecated(since = "4.0.0", note = "Use env::log_str for logging messages.")]
pub fn log(message: impl AsRef<[u8]>) {
    let message = message.as_ref();

    #[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
    eprintln!("{}", String::from_utf8_lossy(message));

    unsafe { sys::log_utf8(message.len() as _, message.as_ptr() as _) }
}

// ###############
// # Storage API #
// ###############
/// Writes key-value into storage.
/// If another key-value existed in the storage with the same key it returns `true`, otherwise `false`.
///
/// # Use cases
/// Storage functions are typically used to upgrade/migrate a contract state, preventing errors like `Cannot deserialize the contract state` after rolling out the breaking changes to the network.
/// For practical examples, see different implementations in [this repository](https://github.com/near-examples/update-migrate-rust).
///
/// # Examples
/// ```
/// use near_sdk::env::{storage_write, storage_read};
///
/// assert!(!storage_write(b"key", b"value"));
/// assert!(storage_write(b"key", b"another_value"));
/// assert_eq!(storage_read(b"key").unwrap(), b"another_value");
/// ```
/// Example of usage [here](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/near-contract-standards/src/upgrade/mod.rs#L63)
pub fn storage_write(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> bool {
    let key = key.as_ref();
    let value = value.as_ref();
    match unsafe {
        sys::storage_write(
            key.len() as _,
            key.as_ptr() as _,
            value.len() as _,
            value.as_ptr() as _,
            EVICTED_REGISTER,
        )
    } {
        0 => false,
        1 => true,
        _ => abort(),
    }
}
/// Reads the value stored under the given key.
///
/// # Use cases
///
/// Storage functions are typically used to upgrade/migrate a contract state, preventing errors like `Cannot deserialize the contract state` after rolling out the breaking changes to the network.
///
/// For practical examples, see different implementations in [`near-examples/update-migrate-rust` repo](https://github.com/near-examples/update-migrate-rust).
///
/// # Examples
/// ```
/// use near_sdk::env::{storage_write, storage_read};
///
/// assert!(storage_read(b"key").is_none());
/// storage_write(b"key", b"value");
/// assert_eq!(storage_read(b"key").unwrap(), b"value");
/// ```
///
/// Another example:
/// - [near-contract-standards/src/upgrade](https://github.com/near/near-sdk-rs/blob/746e4280a7e25b2036bd4e2f2c186cd76e1a7cde/near-contract-standards/src/upgrade/mod.rs?plain=1#L77)
pub fn storage_read(key: impl AsRef<[u8]>) -> Option<Vec<u8>> {
    let key = key.as_ref();
    match unsafe { sys::storage_read(key.len() as _, key.as_ptr() as _, ATOMIC_OP_REGISTER) } {
        0 => None,
        1 => Some(expect_register(read_register(ATOMIC_OP_REGISTER))),
        _ => abort(),
    }
}
/// Removes the value stored under the given key.
/// If key-value existed returns `true`, otherwise `false`.
///
/// # Use cases
/// Storage functions are typically used to upgrade/migrate a contract state, preventing errors like `Cannot deserialize the contract state` after rolling out the breaking changes to the network.
/// For practical examples, see different implementations in [this repository](https://github.com/near-examples/update-migrate-rust).
///
/// # Examples
/// ```
/// use near_sdk::env::{storage_write, storage_remove};
///
/// assert_eq!(storage_remove(b"key"), false);
/// storage_write(b"key", b"value");
/// assert_eq!(storage_remove(b"key"), true);
/// ```
/// Example of usage [here](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/near-contract-standards/src/upgrade/mod.rs#L79)
pub fn storage_remove(key: impl AsRef<[u8]>) -> bool {
    let key = key.as_ref();
    match unsafe { sys::storage_remove(key.len() as _, key.as_ptr() as _, EVICTED_REGISTER) } {
        0 => false,
        1 => true,
        _ => abort(),
    }
}
/// Reads the most recent value that was evicted with `storage_write` or `storage_remove` command.
///
/// # Use cases
/// Storage functions are typically used to upgrade/migrate a contract state, preventing errors like `Cannot deserialize the contract state` after rolling out the breaking changes to the network.
/// For practical examples, see different implementations in [this repository](https://github.com/near-examples/update-migrate-rust).
///
/// # Examples
/// ```
/// use near_sdk::env::{storage_write, storage_remove, storage_get_evicted};
///
/// assert_eq!(storage_get_evicted(), None);
/// ```
pub fn storage_get_evicted() -> Option<Vec<u8>> {
    read_register(EVICTED_REGISTER)
}
/// Checks if there is a key-value in the storage.
///
/// # Use cases
/// Storage functions are typically used to upgrade/migrate a contract state, preventing errors like `Cannot deserialize the contract state` after rolling out the breaking changes to the network.
/// For practical examples, see different implementations in [this repository](https://github.com/near-examples/update-migrate-rust).
///
/// # Examples
/// ```
/// use near_sdk::env::{storage_write, storage_has_key};
///
/// assert_eq!(storage_has_key(b"key"), false);
/// storage_write(b"key", b"value");
/// assert_eq!(storage_has_key(b"key"), true);
/// ```
pub fn storage_has_key(key: impl AsRef<[u8]>) -> bool {
    let key = key.as_ref();
    match unsafe { sys::storage_has_key(key.len() as _, key.as_ptr() as _) } {
        0 => false,
        1 => true,
        _ => abort(),
    }
}

// ############################################
// # Saving and loading of the contract state #
// ############################################
/// Load the state of the given object.
pub fn state_read<T: borsh::BorshDeserialize>() -> Option<T> {
    storage_read(STATE_KEY).map(|data| {
        T::try_from_slice(&data)
            .unwrap_or_else(|_| panic_str("Cannot deserialize the contract state."))
    })
}

/// Writes the specified state to storage.
pub fn state_write<T: borsh::BorshSerialize>(state: &T) {
    let data = match borsh::to_vec(state) {
        Ok(serialized) => serialized,
        Err(_) => panic_str("Cannot serialize the contract state."),
    };
    storage_write(STATE_KEY, &data);
}
/// Returns `true` if the contract state exists and `false` otherwise.
pub fn state_exists() -> bool {
    storage_has_key(STATE_KEY)
}

// #####################################
// # Parameters exposed by the runtime #
// #####################################

/// Price per 1 byte of storage from mainnet genesis config.
/// TODO: will be using the host function when it will be available.
///
/// # Examples
/// ```
/// use near_sdk::env::storage_byte_cost;
/// use near_sdk::NearToken;
///
/// assert_eq!(storage_byte_cost(), NearToken::from_yoctonear(10000000000000000000));
/// ```
/// Example of usage [here](https://github.com/near/near-sdk-rs/blob/189897180649bce47aefa4e5af03664ee525508d/near-contract-standards/src/fungible_token/storage_impl.rs#L105), [here](https://github.com/near/near-sdk-rs/blob/master/near-contract-standards/src/non_fungible_token/utils.rs) and [here](https://github.com/near/near-sdk-rs/blob/master/examples/fungible-token/tests/workspaces.rs)
pub fn storage_byte_cost() -> NearToken {
    NearToken::from_yoctonear(10_000_000_000_000_000_000u128)
}

// ##################
// # Helper methods #
// ##################

/// Returns `true` if the given account ID is valid and `false` otherwise.
///
/// # Examples
///
/// ```
/// use near_sdk::env::is_valid_account_id;
///
/// assert_eq!(is_valid_account_id(b"test.near"), true);
/// assert_eq!(is_valid_account_id(b"test!.%.near"), false);
/// ```
pub fn is_valid_account_id(account_id: &[u8]) -> bool {
    if (account_id.len() as u64) < MIN_ACCOUNT_ID_LEN
        || (account_id.len() as u64) > MAX_ACCOUNT_ID_LEN
    {
        return false;
    }

    // NOTE: We don't want to use Regex here, because it requires extra time to compile it.
    // The valid account ID regex is /^(([a-z\d]+[-_])*[a-z\d]+\.)*([a-z\d]+[-_])*[a-z\d]+$/
    // Instead the implementation is based on the previous character checks.

    // We can safely assume that last char was a separator.
    let mut last_char_is_separator = true;

    for c in account_id {
        let current_char_is_separator = match *c {
            b'a'..=b'z' | b'0'..=b'9' => false,
            b'-' | b'_' | b'.' => true,
            _ => return false,
        };
        if current_char_is_separator && last_char_is_separator {
            return false;
        }
        last_char_is_separator = current_char_is_separator;
    }
    // The account can't end as separator.
    !last_char_is_separator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_account_id_strings() {
        // Valid
        for account_id in &[
            "aa",
            "a-a",
            "a-aa",
            "100",
            "0o",
            "com",
            "near",
            "bowen",
            "b-o_w_e-n",
            "b.owen",
            "bro.wen",
            "a.ha",
            "a.b-a.ra",
            "system",
            "over.9000",
            "google.com",
            "illia.cheapaccounts.near",
            "0o0ooo00oo00o",
            "alex-skidanov",
            "10-4.8-2",
            "b-o_w_e-n",
            "no_lols",
            "0123456789012345678901234567890123456789012345678901234567890123",
            // Valid, but can't be created
            "near.a",
            "a.a",
        ] {
            assert!(
                is_valid_account_id(account_id.as_ref()),
                "Valid account id {:?} marked invalid",
                account_id
            );
        }

        // Invalid
        for account_id in &[
            "",
            "a",
            "A",
            "Abc",
            "-near",
            "near-",
            "-near-",
            "near.",
            ".near",
            "near@",
            "@near",
            "неар",
            "@@@@@",
            "0__0",
            "0_-_0",
            "0_-_0",
            "..",
            "a..near",
            "nEar",
            "_bowen",
            "hello world",
            "abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz",
            "01234567890123456789012345678901234567890123456789012345678901234",
            // `@` separators are banned now
            "some-complex-address@gmail.com",
            "sub.buy_d1gitz@atata@b0-rg.c_0_m",
        ] {
            assert!(
                !is_valid_account_id(account_id.as_ref()),
                "Invalid account id {:?} marked valid",
                account_id
            );
        }
    }

    #[test]
    fn test_is_valid_account_id_binary() {
        assert!(!is_valid_account_id(&[]));
        assert!(!is_valid_account_id(&[0]));
        assert!(!is_valid_account_id(&[0, 1]));
        assert!(!is_valid_account_id(&[0, 1, 2]));
        assert!(is_valid_account_id(b"near"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn hash_smoke_tests() {
        assert_eq!(
            &super::sha256_array(b"some value"),
            hex::decode("ab3d07f3169ccbd0ed6c4b45de21519f9f938c72d24124998aab949ce83bb51b")
                .unwrap()
                .as_slice()
        );

        assert_eq!(
            &super::keccak256_array(b"some value"),
            hex::decode("f928dfb5fc72b3bbfb9a5ccb0ee9843b27b4ac1ebc25a6f6f783e23ebd47ef1f")
                .unwrap()
                .as_slice()
        );

        assert_eq!(
            &super::keccak512_array(b"some value"),
            hex::decode("3e38d140a85123374ee63ec208973aa39b87349d17ccac948a2493e18b18b5913220cd174b4f511aa97977009e16be485fc94f5e2743cb9bb0579d35ab410583")
                .unwrap()
                .as_slice()
        );

        assert_eq!(
            &super::ripemd160_array(b"some value"),
            hex::decode("09f025fed704e1ecac8f88b2bda3e56876da03ac").unwrap().as_slice()
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn random_seed_smoke_test() {
        crate::testing_env!(crate::test_utils::VMContextBuilder::new()
            .random_seed([8; 32])
            .build());

        assert_eq!(super::random_seed(), [8; 32]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "unstable")]
    #[test]
    fn test_ecrecover() {
        use crate::test_utils::test_env;
        use hex::FromHex;
        use serde::de::Error;
        use serde::{Deserialize, Deserializer};
        use serde_json::from_slice;
        use std::fmt::Display;

        #[derive(Deserialize)]
        struct EcrecoverTest {
            #[serde(with = "hex::serde")]
            m: [u8; 32],
            v: u8,
            #[serde(with = "hex::serde")]
            sig: [u8; 64],
            mc: bool,
            #[serde(deserialize_with = "deserialize_option_hex")]
            res: Option<[u8; 64]>,
        }

        fn deserialize_option_hex<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            D: Deserializer<'de>,
            T: FromHex,
            <T as FromHex>::Error: Display,
        {
            Deserialize::deserialize(deserializer)
                .map(|v: Option<&str>| v.map(FromHex::from_hex).transpose().map_err(Error::custom))
                .and_then(|v| v)
        }

        test_env::setup_free();
        for EcrecoverTest { m, v, sig, mc, res } in
            from_slice::<'_, Vec<_>>(include_bytes!("../../tests/ecrecover-tests.json")).unwrap()
        {
            assert_eq!(super::ecrecover(&m, &sig, v, mc), res);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn signer_public_key() {
        let key: PublicKey =
            "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse().unwrap();

        crate::testing_env!(crate::test_utils::VMContextBuilder::new()
            .signer_account_pk(key.clone())
            .build());
        assert_eq!(super::signer_account_pk(), key);
    }

    #[test]
    fn ed25519_verify() {
        const SIGNATURE: [u8; 64] = [
            145, 193, 203, 18, 114, 227, 14, 117, 33, 213, 121, 66, 130, 14, 25, 4, 36, 120, 46,
            142, 226, 215, 7, 66, 122, 112, 97, 30, 249, 135, 61, 165, 221, 249, 252, 23, 105, 40,
            56, 70, 31, 152, 236, 141, 154, 122, 207, 20, 75, 118, 79, 90, 168, 6, 221, 122, 213,
            29, 126, 196, 216, 104, 191, 6,
        ];

        const BAD_SIGNATURE: [u8; 64] = [1; 64];

        // create a forged signature with the `s` scalar not properly reduced
        // https://docs.rs/ed25519/latest/src/ed25519/lib.rs.html#302
        const FORGED_SIGNATURE: [u8; 64] = {
            let mut sig = SIGNATURE;
            sig[63] = 0b1110_0001;
            sig
        };

        const PUBLIC_KEY: [u8; 32] = [
            32, 122, 6, 120, 146, 130, 30, 37, 215, 112, 241, 251, 160, 196, 124, 17, 255, 75, 129,
            62, 84, 22, 46, 206, 158, 184, 57, 224, 118, 35, 26, 182,
        ];

        // create a forged public key to force a PointDecompressionError
        // https://docs.rs/ed25519-dalek/latest/src/ed25519_dalek/public.rs.html#142
        const FORGED_PUBLIC_KEY: [u8; 32] = {
            let mut key = PUBLIC_KEY;
            key[31] = 0b1110_0001;
            key
        };

        // 32 bytes message
        const MESSAGE: [u8; 32] = [
            107, 97, 106, 100, 108, 102, 107, 106, 97, 108, 107, 102, 106, 97, 107, 108, 102, 106,
            100, 107, 108, 97, 100, 106, 102, 107, 108, 106, 97, 100, 115, 107,
        ];

        assert!(super::ed25519_verify(&SIGNATURE, MESSAGE, &PUBLIC_KEY));
        assert!(!super::ed25519_verify(&BAD_SIGNATURE, MESSAGE, &FORGED_PUBLIC_KEY));
        assert!(!super::ed25519_verify(&SIGNATURE, MESSAGE, &FORGED_PUBLIC_KEY));
        assert!(!super::ed25519_verify(&FORGED_SIGNATURE, MESSAGE, &PUBLIC_KEY));
    }

    #[test]
    pub fn alt_bn128_g1_multiexp() {
        // Originated from https://github.com/near/nearcore/blob/8cd095ffc98a6507ed2d2a8982a6a3e42ebc1b62/runtime/near-test-contracts/estimator-contract/src/lib.rs#L557-L720
        let buffer = [
            16, 238, 91, 161, 241, 22, 172, 158, 138, 252, 202, 212, 136, 37, 110, 231, 118, 220,
            8, 45, 14, 153, 125, 217, 227, 87, 238, 238, 31, 138, 226, 8, 238, 185, 12, 155, 93,
            126, 144, 248, 200, 177, 46, 245, 40, 162, 169, 80, 150, 211, 157, 13, 10, 36, 44, 232,
            173, 32, 32, 115, 123, 2, 9, 47, 190, 148, 181, 91, 69, 6, 83, 40, 65, 222, 251, 70,
            81, 73, 60, 142, 130, 217, 176, 20, 69, 75, 40, 167, 41, 180, 244, 5, 142, 215, 135,
            35,
        ];

        assert_eq!(
            super::alt_bn128_g1_multiexp(buffer),
            vec![
                150, 94, 159, 52, 239, 226, 181, 150, 77, 86, 90, 186, 102, 219, 243, 204, 36, 128,
                164, 209, 106, 6, 62, 124, 235, 104, 223, 195, 30, 204, 42, 20, 13, 158, 14, 197,
                133, 73, 43, 171, 28, 68, 82, 116, 244, 164, 36, 251, 244, 8, 234, 40, 118, 55,
                216, 187, 242, 39, 213, 160, 192, 184, 28, 23
            ]
        );
    }

    #[test]
    pub fn alt_bn128_g1_sum() {
        // Originated from https://github.com/near/nearcore/blob/8cd095ffc98a6507ed2d2a8982a6a3e42ebc1b62/runtime/near-test-contracts/estimator-contract/src/lib.rs#L557-L720
        let buffer = [
            0, 11, 49, 94, 29, 152, 111, 116, 138, 248, 2, 184, 8, 159, 80, 169, 45, 149, 48, 32,
            49, 37, 6, 133, 105, 171, 194, 120, 44, 195, 17, 180, 35, 137, 154, 4, 192, 211, 244,
            93, 200, 2, 44, 0, 64, 26, 108, 139, 147, 88, 235, 242, 23, 253, 52, 110, 236, 67, 99,
            176, 2, 186, 198, 228, 25,
        ];

        assert_eq!(
            super::alt_bn128_g1_sum(buffer),
            vec![
                11, 49, 94, 29, 152, 111, 116, 138, 248, 2, 184, 8, 159, 80, 169, 45, 149, 48, 32,
                49, 37, 6, 133, 105, 171, 194, 120, 44, 195, 17, 180, 35, 137, 154, 4, 192, 211,
                244, 93, 200, 2, 44, 0, 64, 26, 108, 139, 147, 88, 235, 242, 23, 253, 52, 110, 236,
                67, 99, 176, 2, 186, 198, 228, 25
            ]
        );
    }

    #[test]
    pub fn alt_bn128_pairing_check() {
        // Taken from https://github.com/near/nearcore/blob/8cd095ffc98a6507ed2d2a8982a6a3e42ebc1b62/runtime/near-vm-runner/src/logic/tests/alt_bn128.rs#L239-L250
        let valid_pair = [
            117, 10, 217, 99, 113, 78, 234, 67, 183, 90, 26, 58, 200, 86, 195, 123, 42, 184, 213,
            88, 224, 248, 18, 200, 108, 6, 181, 6, 28, 17, 99, 7, 36, 134, 53, 115, 192, 180, 3,
            113, 76, 227, 174, 147, 50, 174, 79, 74, 151, 195, 172, 10, 211, 210, 26, 92, 117, 246,
            65, 237, 168, 104, 16, 4, 1, 26, 3, 219, 6, 13, 193, 115, 77, 230, 27, 13, 242, 214,
            195, 9, 213, 99, 135, 12, 160, 202, 114, 135, 175, 42, 116, 172, 79, 234, 26, 41, 212,
            111, 192, 129, 124, 112, 57, 107, 38, 244, 230, 222, 240, 36, 65, 238, 133, 188, 19,
            43, 148, 59, 205, 40, 161, 179, 173, 228, 88, 169, 231, 29, 17, 67, 163, 51, 165, 187,
            101, 44, 250, 24, 68, 101, 92, 128, 203, 190, 51, 85, 9, 43, 58, 136, 68, 180, 92, 110,
            185, 168, 107, 129, 45, 30, 187, 22, 100, 17, 75, 93, 216, 125, 23, 212, 11, 186, 199,
            204, 1, 140, 133, 11, 82, 44, 65, 222, 20, 26, 48, 26, 132, 220, 25, 213, 93, 25, 79,
            176, 4, 149, 151, 243, 11, 131, 253, 233, 121, 38, 222, 15, 118, 117, 200, 214, 175,
            233, 130, 181, 193, 167, 255, 153, 169, 240, 207, 235, 28, 31, 83, 74, 69, 179, 6, 150,
            72, 67, 74, 166, 130, 83, 82, 115, 123, 111, 208, 221, 64, 43, 237, 213, 186, 235, 7,
            56, 251, 179, 95, 233, 159, 23, 109, 173, 85, 103, 8, 165, 235, 226, 218, 79, 72, 120,
            172, 251, 20, 83, 121, 201, 140, 98, 170, 246, 121, 218, 19, 115, 42, 135, 60, 239, 30,
            32, 49, 170, 171, 204, 196, 197, 160, 158, 168, 47, 23, 110, 139, 123, 222, 222, 245,
            98, 125, 208, 70, 39, 110, 186, 146, 254, 66, 185, 118, 3, 78, 32, 47, 179, 197, 93,
            79, 240, 204, 78, 236, 133, 213, 173, 117, 94, 63, 154, 68, 89, 236, 138, 0, 247, 242,
            212, 245, 33, 249, 0, 35, 246, 233, 0, 124, 86, 198, 162, 201, 54, 19, 26, 196, 75,
            254, 71, 70, 238, 51, 2, 23, 185, 152, 139, 134, 65, 107, 129, 114, 244, 47, 251, 240,
            80, 193, 23,
        ];
        assert!(super::alt_bn128_pairing_check(valid_pair));

        // Taken from https://github.com/near/nearcore/blob/8cd095ffc98a6507ed2d2a8982a6a3e42ebc1b62/runtime/near-vm-runner/src/logic/tests/alt_bn128.rs#L254-L265
        let invalid_pair = [
            117, 10, 217, 99, 113, 78, 234, 67, 183, 90, 26, 58, 200, 86, 195, 123, 42, 184, 213,
            88, 224, 248, 18, 200, 108, 6, 181, 6, 28, 17, 99, 7, 36, 134, 53, 115, 192, 180, 3,
            113, 76, 227, 174, 147, 50, 174, 79, 74, 151, 195, 172, 10, 211, 210, 26, 92, 117, 246,
            65, 237, 168, 104, 16, 4, 1, 26, 3, 219, 6, 13, 193, 115, 77, 230, 27, 13, 242, 214,
            195, 9, 213, 99, 135, 12, 160, 202, 114, 135, 175, 42, 116, 172, 79, 234, 26, 41, 212,
            111, 192, 129, 124, 112, 57, 107, 38, 244, 230, 222, 240, 36, 65, 238, 133, 188, 19,
            43, 148, 59, 205, 40, 161, 179, 173, 228, 88, 169, 231, 29, 17, 67, 163, 51, 165, 187,
            101, 44, 250, 24, 68, 101, 92, 128, 203, 190, 51, 85, 9, 43, 58, 136, 68, 180, 92, 110,
            185, 168, 107, 129, 45, 30, 187, 22, 100, 17, 75, 93, 216, 125, 23, 212, 11, 186, 199,
            204, 1, 140, 133, 11, 82, 44, 65, 222, 20, 26, 48, 26, 132, 220, 25, 213, 93, 25, 117,
            10, 217, 99, 113, 78, 234, 67, 183, 90, 26, 58, 200, 86, 195, 123, 42, 184, 213, 88,
            224, 248, 18, 200, 108, 6, 181, 6, 28, 17, 99, 7, 36, 134, 53, 115, 192, 180, 3, 113,
            76, 227, 174, 147, 50, 174, 79, 74, 151, 195, 172, 10, 211, 210, 26, 92, 117, 246, 65,
            237, 168, 104, 16, 4, 109, 173, 85, 103, 8, 165, 235, 226, 218, 79, 72, 120, 172, 251,
            20, 83, 121, 201, 140, 98, 170, 246, 121, 218, 19, 115, 42, 135, 60, 239, 30, 32, 49,
            170, 171, 204, 196, 197, 160, 158, 168, 47, 23, 110, 139, 123, 222, 222, 245, 98, 125,
            208, 70, 39, 110, 186, 146, 254, 66, 185, 118, 3, 78, 32, 47, 179, 197, 93, 79, 240,
            204, 78, 236, 133, 213, 173, 117, 94, 63, 154, 68, 89, 236, 138, 0, 247, 242, 212, 245,
            33, 249, 0, 35, 246, 233, 0, 124, 86, 198, 162, 201, 54, 19, 26, 196, 75, 254, 71, 70,
            238, 51, 2, 23, 185, 152, 139, 134, 65, 107, 129, 114, 244, 47, 251, 240, 80, 193, 23,
        ];

        assert!(!super::alt_bn128_pairing_check(invalid_pair));
    }
    #[test]
    fn bls12381_p1_sum_0_100() {
        let buffer: [u8; 0] = [];
        for _ in 0..100 {
            let result = super::bls12381_p1_sum(buffer);
            assert!(!result.is_empty(), "Expected a non-empty result from bls12381_p1_sum");
        }
    }

    #[test]
    fn bls12381_p1_sum_50_100() {
        let buffer: [[u8; 2 * 97]; 25] = [[
            0, 18, 25, 108, 90, 67, 214, 146, 36, 216, 113, 51, 137, 40, 95, 38, 185, 143, 134,
            238, 145, 10, 179, 221, 102, 142, 65, 55, 56, 40, 32, 3, 204, 91, 115, 87, 175, 154,
            122, 245, 75, 183, 19, 214, 34, 85, 232, 15, 86, 6, 186, 129, 2, 191, 190, 234, 68, 22,
            183, 16, 199, 62, 140, 206, 48, 50, 195, 28, 98, 105, 196, 73, 6, 248, 172, 79, 120,
            116, 206, 153, 251, 23, 85, 153, 146, 72, 101, 40, 150, 56, 132, 206, 66, 154, 153, 47,
            238, 0, 0, 1, 16, 16, 152, 245, 195, 152, 147, 118, 87, 102, 175, 69, 18, 160, 199, 78,
            27, 184, 155, 199, 230, 253, 241, 78, 62, 115, 55, 210, 87, 204, 15, 148, 101, 129,
            121, 216, 51, 32, 185, 159, 49, 255, 148, 205, 43, 172, 3, 225, 169, 249, 244, 76, 162,
            205, 171, 79, 67, 161, 163, 238, 52, 112, 253, 249, 11, 47, 194, 40, 235, 59, 112, 159,
            205, 114, 240, 20, 131, 138, 200, 42, 109, 121, 122, 238, 254, 217, 160, 128, 75, 34,
            237, 28, 232, 247,
        ]; 25];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        for _ in 0..100 {
            let result = super::bls12381_p1_sum(&flat);
            assert!(!result.is_empty(), "Expected a non-empty result from bls12381_p1_sum");
        }
    }

    #[test]
    fn bls12381_p2_sum_0_100() {
        let buffer: [u8; 0] = [];
        for _ in 0..100 {
            let result = super::bls12381_p2_sum(buffer);
            assert!(!result.is_empty(), "Expected a non-empty result from bls12381_p2_sum");
        }
    }

    #[test]
    fn bls12381_p2_sum_50_100() {
        let buffer: [[u8; 2 * 193]; 25] = [[
            0, 12, 199, 10, 88, 127, 70, 82, 3, 157, 129, 23, 182, 16, 56, 88, 173, 205, 151, 40,
            246, 174, 190, 35, 5, 120, 56, 154, 98, 218, 0, 66, 183, 98, 59, 28, 4, 54, 115, 79,
            70, 60, 253, 209, 135, 210, 9, 3, 36, 24, 192, 173, 166, 53, 27, 112, 102, 31, 5, 51,
            101, 222, 174, 86, 145, 7, 152, 189, 42, 206, 110, 43, 246, 186, 65, 146, 209, 162, 41,
            150, 127, 106, 246, 202, 28, 154, 138, 17, 235, 192, 162, 50, 52, 78, 224, 246, 214, 7,
            155, 165, 13, 37, 17, 99, 27, 32, 182, 214, 243, 132, 30, 97, 110, 157, 17, 182, 142,
            195, 54, 140, 214, 1, 41, 217, 212, 120, 122, 181, 108, 78, 145, 69, 163, 137, 39, 229,
            28, 156, 214, 39, 29, 73, 61, 147, 136, 9, 245, 11, 215, 190, 237, 178, 51, 40, 129,
            143, 159, 253, 175, 219, 109, 166, 164, 221, 128, 197, 169, 4, 138, 184, 177, 84, 223,
            60, 173, 147, 140, 206, 222, 130, 159, 17, 86, 247, 105, 217, 225, 73, 121, 30, 142,
            12, 217, 0, 9, 174, 177, 12, 55, 43, 94, 241, 1, 6, 117, 198, 164, 118, 47, 218, 51,
            99, 100, 137, 194, 59, 88, 28, 117, 34, 5, 137, 175, 188, 12, 196, 98, 73, 249, 33,
            238, 160, 45, 209, 183, 97, 224, 54, 255, 219, 174, 34, 25, 47, 165, 216, 115, 47, 249,
            243, 142, 11, 28, 241, 46, 173, 253, 38, 8, 240, 199, 163, 154, 206, 215, 116, 104, 55,
            131, 58, 226, 83, 187, 87, 239, 156, 13, 152, 164, 182, 158, 235, 41, 80, 144, 25, 23,
            233, 157, 30, 23, 72, 130, 205, 211, 85, 30, 12, 230, 23, 136, 97, 255, 131, 225, 149,
            254, 203, 207, 253, 83, 166, 123, 111, 16, 180, 67, 30, 66, 62, 40, 164, 128, 50, 127,
            235, 231, 2, 118, 3, 111, 96, 187, 156, 153, 207, 118, 51, 2, 210, 37, 68, 118, 0, 212,
            159, 147, 43, 157, 211, 202, 30, 105, 89, 105, 122, 166, 3, 231, 77, 134, 102, 104, 26,
            45, 202, 129, 96, 195, 133, 118, 104, 174, 7, 68, 64, 54, 102, 25, 235, 137, 32, 37,
            108, 78, 74,
        ]; 25];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_p2_sum(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_p2_sum");
    }

    #[test]
    fn bls12381_g1_multiexp_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_g1_multiexp(buffer);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_g1_multiexp");
    }

    #[test]
    fn bls12381_g1_multiexp_50_100() {
        let buffer: [[u8; 96 + 32]; 50] = [[
            23, 241, 211, 167, 49, 151, 215, 148, 38, 149, 99, 140, 79, 169, 172, 15, 195, 104,
            140, 79, 151, 116, 185, 5, 161, 78, 58, 63, 23, 27, 172, 88, 108, 85, 232, 63, 249,
            122, 26, 239, 251, 58, 240, 10, 219, 34, 198, 187, 8, 179, 244, 129, 227, 170, 160,
            241, 160, 158, 48, 237, 116, 29, 138, 228, 252, 245, 224, 149, 213, 208, 10, 246, 0,
            219, 24, 203, 44, 4, 179, 237, 208, 60, 199, 68, 162, 136, 138, 228, 12, 170, 35, 41,
            70, 197, 231, 225, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255,
        ]; 50];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_g1_multiexp(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_g1_multiexp");
    }

    #[test]
    fn bls12381_g2_multiexp_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_g2_multiexp(buffer);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_g2_multiexp");
    }

    #[test]
    fn bls12381_g2_multiexp_50_100() {
        let buffer: [[u8; 192 + 32]; 50] = [[
            19, 224, 43, 96, 82, 113, 159, 96, 125, 172, 211, 160, 136, 39, 79, 101, 89, 107, 208,
            208, 153, 32, 182, 26, 181, 218, 97, 187, 220, 127, 80, 73, 51, 76, 241, 18, 19, 148,
            93, 87, 229, 172, 125, 5, 93, 4, 43, 126, 2, 74, 162, 178, 240, 143, 10, 145, 38, 8, 5,
            39, 45, 197, 16, 81, 198, 228, 122, 212, 250, 64, 59, 2, 180, 81, 11, 100, 122, 227,
            209, 119, 11, 172, 3, 38, 168, 5, 187, 239, 212, 128, 86, 200, 193, 33, 189, 184, 6, 6,
            196, 160, 46, 167, 52, 204, 50, 172, 210, 176, 43, 194, 139, 153, 203, 62, 40, 126,
            133, 167, 99, 175, 38, 116, 146, 171, 87, 46, 153, 171, 63, 55, 13, 39, 92, 236, 29,
            161, 170, 169, 7, 95, 240, 95, 121, 190, 12, 229, 213, 39, 114, 125, 110, 17, 140, 201,
            205, 198, 218, 46, 53, 26, 173, 253, 155, 170, 140, 189, 211, 167, 109, 66, 154, 105,
            81, 96, 209, 44, 146, 58, 201, 204, 59, 172, 162, 137, 225, 147, 84, 134, 8, 184, 40,
            1, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ]; 50];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_g2_multiexp(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_g2_multiexp");
    }

    #[test]
    fn bls12381_map_fp_to_g1_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_map_fp_to_g1(buffer);
        assert!(result.is_empty(), "Expected an empty result from bls12381_map_fp_to_g1");
    }

    #[test]
    fn bls12381_map_fp_to_g1_50_100() {
        let buffer: [[u8; 48]; 50] = [[
            20, 64, 110, 91, 251, 146, 9, 37, 106, 56, 32, 135, 154, 41, 172, 47, 98, 214, 172,
            168, 35, 36, 191, 58, 226, 170, 125, 60, 84, 121, 32, 67, 189, 140, 121, 31, 204, 219,
            8, 12, 26, 82, 220, 104, 184, 182, 147, 80,
        ]; 50];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_map_fp_to_g1(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_map_fp_to_g1");
    }

    #[test]
    fn bls12381_map_fp2_to_g2_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_map_fp2_to_g2(buffer);
        assert!(result.is_empty(), "Expected an empty result from bls12381_map_fp2_to_g2");
    }

    #[test]
    fn bls12381_map_fp2_to_g2_10_100() {
        let buffer: [[u8; 96]; 10] = [[
            14, 136, 91, 179, 57, 150, 225, 47, 7, 218, 105, 7, 62, 44, 12, 200, 128, 188, 142,
            255, 38, 210, 167, 36, 41, 158, 177, 45, 84, 244, 188, 242, 111, 71, 72, 187, 2, 14,
            128, 167, 227, 121, 74, 123, 14, 71, 166, 65, 20, 64, 110, 91, 251, 146, 9, 37, 106,
            56, 32, 135, 154, 41, 172, 47, 98, 214, 172, 168, 35, 36, 191, 58, 226, 170, 125, 60,
            84, 121, 32, 67, 189, 140, 121, 31, 204, 219, 8, 12, 26, 82, 220, 104, 184, 182, 147,
            80,
        ]; 10];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_map_fp2_to_g2(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_map_fp2_to_g2");
    }

    #[test]
    fn bls12381_pairing_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_pairing_check(buffer);
        assert!(result, "Expected result to be true");
    }

    #[test]
    fn bls12381_pairing_valid_check() {
        // Valid test vector (should return true)
        let valid_input = hex::decode("085fad8696122c8a421033164e6a71d9adb3882933beba2c14dcad9bfd4badb30b49306c59a7a7837b72e02993f5a4ad025871da31a9be44cd3a46365038ef6f3658fc65ff3064e348083b2de4d983c7436f486f6e9de272fa0db7dfa543656811f7dbc8c5b084e2daf685536a2d155d69c7683b811c840e4167a5c966bad4eebfdb757ef9caa63ffde16727fa5c15ac0b15a2802624e85d6987eb53a69714401adfd5ca5e6151a8e9c0790dfc4494ea77ad32b66e95da7f615ee2fe7b6594f00493deb2392b4159afc07b69000f9b097ecca94bf5a46cb13f95dabdd9a40a2e207c077059c821caa29a40930b4b757f11404dcfe5e92c69acdbf3667651d5adf6856956805693fb945d83c5cf158371536814442ff31d6ad1b834a4ab13ad9917f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb114d1d6855d545a8aa7d76c8cf2e21f267816aef1db507c96655b9d5caac42364e6f38ba0ecb751bad54dcd6b939c2ca0f968bd243908ff3e5fa1ab3f31e078197e58ace562bbe8b5a271d5fba50237da0c8fe65e7b5771cc0a86fd57f32347e15a26d1f5d56c472d019eea2539e58db00c49aa5d0a9663838903fddbe436b5b157e83b35d1a4e5f89f78127f35dacf005a2854c7f36818c137070d1342bba362b5d0c7daed605fcc739df577c33bd6ab6e07ab4a97beee81aa57c8d41f447440eeaf1f595b7b57457d7792b4bc14be74d0038f7ac3767a9c61fecaa02c3d07982c02995f22f66c05b8eb3b9facd5571").unwrap();

        let result = super::bls12381_pairing_check(&valid_input);
        assert!(result, "Expected valid pairing check to return true");
    }

    #[test]
    fn bls12381_pairing_5_100() {
        let buffer: [[u8; 288]; 5] = [[
            23, 241, 211, 167, 49, 151, 215, 148, 38, 149, 99, 140, 79, 169, 172, 15, 195, 104,
            140, 79, 151, 116, 185, 5, 161, 78, 58, 63, 23, 27, 172, 88, 108, 85, 232, 63, 249,
            122, 26, 239, 251, 58, 240, 10, 219, 34, 198, 187, 8, 179, 244, 129, 227, 170, 160,
            241, 160, 158, 48, 237, 116, 29, 138, 228, 252, 245, 224, 149, 213, 208, 10, 246, 0,
            219, 24, 203, 44, 4, 179, 237, 208, 60, 199, 68, 162, 136, 138, 228, 12, 170, 35, 41,
            70, 197, 231, 225, 19, 224, 43, 96, 82, 113, 159, 96, 125, 172, 211, 160, 136, 39, 79,
            101, 89, 107, 208, 208, 153, 32, 182, 26, 181, 218, 97, 187, 220, 127, 80, 73, 51, 76,
            241, 18, 19, 148, 93, 87, 229, 172, 125, 5, 93, 4, 43, 126, 2, 74, 162, 178, 240, 143,
            10, 145, 38, 8, 5, 39, 45, 197, 16, 81, 198, 228, 122, 212, 250, 64, 59, 2, 180, 81,
            11, 100, 122, 227, 209, 119, 11, 172, 3, 38, 168, 5, 187, 239, 212, 128, 86, 200, 193,
            33, 189, 184, 6, 6, 196, 160, 46, 167, 52, 204, 50, 172, 210, 176, 43, 194, 139, 153,
            203, 62, 40, 126, 133, 167, 99, 175, 38, 116, 146, 171, 87, 46, 153, 171, 63, 55, 13,
            39, 92, 236, 29, 161, 170, 169, 7, 95, 240, 95, 121, 190, 12, 229, 213, 39, 114, 125,
            110, 17, 140, 201, 205, 198, 218, 46, 53, 26, 173, 253, 155, 170, 140, 189, 211, 167,
            109, 66, 154, 105, 81, 96, 209, 44, 146, 58, 201, 204, 59, 172, 162, 137, 225, 147, 84,
            134, 8, 184, 40, 1,
        ]; 5];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_pairing_check(&flat);
        assert!(!result, "Expected result to be false");
    }

    #[test]
    fn bls12381_p1_decompress_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_p1_decompress(buffer);
        assert!(result.is_empty(), "Expected an empty result from bls12381_p1_decompress");
    }

    #[test]
    fn bls12381_p1_decompress_50_100() {
        let buffer: [[u8; 48]; 50] = [[
            185, 110, 35, 139, 110, 142, 126, 177, 120, 97, 234, 41, 91, 204, 20, 203, 207, 103,
            224, 112, 176, 18, 102, 59, 68, 107, 137, 231, 10, 71, 183, 63, 198, 228, 242, 206,
            195, 124, 70, 91, 53, 182, 222, 158, 19, 104, 106, 15,
        ]; 50];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_p1_decompress(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_p1_decompress");
    }

    #[test]
    fn bls12381_p2_decompress_0_100() {
        let buffer: [u8; 0] = [];
        let result = super::bls12381_p2_decompress(buffer);
        assert!(result.is_empty(), "Expected an empty result from bls12381_p2_decompress");
    }

    #[test]
    fn bls12381_p2_decompress_50_100() {
        let buffer: [[u8; 96]; 50] = [[
            143, 150, 139, 210, 67, 144, 143, 243, 229, 250, 26, 179, 243, 30, 7, 129, 151, 229,
            138, 206, 86, 43, 190, 139, 90, 39, 29, 95, 186, 80, 35, 125, 160, 200, 254, 101, 231,
            181, 119, 28, 192, 168, 111, 213, 127, 50, 52, 126, 21, 162, 109, 31, 93, 86, 196, 114,
            208, 25, 238, 162, 83, 158, 88, 219, 0, 196, 154, 165, 208, 169, 102, 56, 56, 144, 63,
            221, 190, 67, 107, 91, 21, 126, 131, 179, 93, 26, 78, 95, 137, 247, 129, 39, 243, 93,
            172, 240,
        ]; 50];
        let flat: Vec<u8> = buffer.iter().flat_map(|x| x.iter()).copied().collect();
        let result = super::bls12381_p2_decompress(&flat);
        assert!(!result.is_empty(), "Expected a non-empty result from bls12381_p2_decompress");
    }

    #[test]
    #[cfg(feature = "global-contracts")]
    fn test_global_contract_functions() {
        // Test the global contract promise batch action functions
        // These tests verify the functions can be called without panicking

        let promise_index = super::promise_batch_create(AccountIdRef::new_or_panic("alice.near"));
        let code = vec![0u8; 100]; // Mock contract bytecode
        let code_hash = [0u8; 32]; // Mock 32-byte hash (CryptoHash)
        let account_id = AccountIdRef::new_or_panic("deployer.near");

        // Test deploy_global_contract
        super::promise_batch_action_deploy_global_contract(promise_index, &code);

        // Test deploy_global_contract_by_account_id
        super::promise_batch_action_deploy_global_contract_by_account_id(promise_index, &code);

        // Test use_global_contract
        super::promise_batch_action_use_global_contract(promise_index, &code_hash);

        // Test use_global_contract_by_account_id
        super::promise_batch_action_use_global_contract_by_account_id(promise_index, account_id);
    }

    #[test]
    #[cfg(feature = "global-contracts")]
    fn test_global_contract_edge_cases() {
        // Test with minimal valid inputs
        let promise_index = super::promise_batch_create(AccountIdRef::new_or_panic("alice.near"));

        // Test with single byte code (minimal size)
        super::promise_batch_action_deploy_global_contract(promise_index, [0]);
        super::promise_batch_action_deploy_global_contract_by_account_id(promise_index, [0]);

        // Test with 32-byte hash (standard size for CryptoHash)
        let valid_hash = [0u8; 32];
        super::promise_batch_action_use_global_contract(promise_index, &valid_hash);
    }
}
