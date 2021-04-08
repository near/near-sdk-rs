# Changelog

## `3.1.0`

* Updated dependencies for `near-sdk`
* Introduce trait `IntoStorageKey` and updating all persistent collections to take it instead of `Vec<u8>`.
  It's a non-breaking change.
* Introduce a macro derive `BorshStorageKey` that implements `IntoStorageKey` using borsh serialization. Example:
```rust
use near_sdk::BorshStorageKey;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Records,
    UniqueValues,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct StatusMessage {
    pub records: LookupMap<String, String>,
    pub unique_values: LookupSet<String>,
}

#[near_bindgen]
impl StatusMessage {
    #[init]
    pub fn new() -> Self {
        Self {
            records: LookupMap::new(StorageKey::Records),
            unique_values: LookupSet::new(StorageKey::UniqueValues),
        }
    }
}
```

## `3.0.1`

* Introduced `#[private]` method decorator, that verifies `predecessor_account_id() == current_account_id()`.
  NOTE: Usually, when a contract has to have a callback for a remote cross-contract call, this callback method should
  only be called by the contract itself. It's to avoid someone else calling it and messing the state. Pretty common pattern
  is to have an assert that validates that the direct caller (predecessor account ID) matches to the contract's account (current account ID).
* Added how to build contracts with reproducible builds.
* Added `log!` macro to log a string from a contract similar to `println!` macro.
* Added `test_utils` mod from `near_sdk` that contains a bunch of helper methods and structures, e.g.
    * `test_env` - simple test environment mod used internally.
    * Expanded `testing_env` to be able to pass promise results
    * Added `VMContextBuilder` to help construct a `VMContext` for tests
    * Added `get_logs` method that returns current logs from the contract execution.
    * **TEST_BREAKING** `env::created_receipts` moved to `test_utils::get_created_receipts`.
      `env` shouldn't contain testing methods.
    * Updated a few examples to use `log!` macro
* Added `#[derive(PanicOnDefault)]` that automatically implements `Default` trait that panics when called.
  This is helpful to prevent contracts from being initialized using `Default` by removing boilerplate code.
* Introduce `setup_alloc` macro that generates the same boilerplate as before, but also adds a #[cfg(target_arch = "wasm32")], which prevents the allocator from being used when the contract's main file is used in simulation testing.
* Introduce `Base58CryptoHash` and `CryptoHash` to represent `32` bytes slice of `u8`.
* Introduce `LazyOption` to keep a single large value with lazy deserialization.
* **BREAKING** `#[init]` now checks that the state is not initialized. This is expected behavior. To ignore state check you can call `#[init(ignore_state)]`
* NOTE: `3.0.0` is not published, due to tag conflicts on the `near-sdk-rs` repo.

## `2.0.1`

* Pinned version of `syn` crate to `=1.0.57`, since `1.0.58` introduced a breaking API change.

## `2.0.0`

### Contract changes

* Updated `status-message-collections` to use `LookupMap`
* **BREAKING** Updated `fungible-token` implementation to use `LookupMap`. It changes storage layout.

### API changes

* Introduce `LookupMap` and `LookupSet` that are faster implementations of `UnorderedMap` and `UnorderedSet`, but without support for iterators.
  Most read/lookup/write are done in 1 storage access instead of 2 or 3 for `Unordered*` implementations.
* **BREAKING** `Default` is removed from `near_sdk::collections` to avoid implicit state conflicts.
  Collections should be initialized by explicitly specifying prefix using `new` method.
* **BREAKING** `TreeMap` implementation was updated to use `LookupMap`.
  Previous `TreeMap` implementation was renamed to `LegacyTreeMap` and was deprecated.
  It should only be used if the contract was already deployed and state has to be compatible with the previous implementation.

## `1.0.1`

### Other changes

* Remove requirements for input args types to implement `serde::Serialize` and for return types to implement `serde::Deserialize`.

### Fix

* Bumped dependency version of `near-vm-logic` and `near-runtime-fees` to `2.0.0` that changed `VMLogic` interface.

## `1.0.0`

### Other changes

* Re-export common crates to be reused directly from `near_sdk`.
* Added `ValidAccountId` to `json_types` which validates the input string during deserialization to be a valid account ID.
* Added `Debug` to `Base58PublicKey`.
* Bumped dependency version of `borsh` to `0.7.0`.
* Bumped dependency version of `near-vm-logic` and `near-runtime-fees` to `1.0.0`.
* Implemented Debug trait for Vector collection that can be enabled with `expensive-debug` feature.

### Contract changes

* Use re-exported crate dependencies through `near_sdk` crate.

## `0.11.0`

### API breaking changes

* Renamed `Map` to `UnorderedMap` and `Set` to `UnorderedSet` to reflect that one cannot rely on the order of the elements in them. In this PR and in https://github.com/near/near-sdk-rs/pull/154

### Other changes

* Added ordered tree implementation based on AVL, see `TreeMap`. https://github.com/near/near-sdk-rs/pull/154

* Made module generated by `ext_contract` macro public, providing more flexibility for the usage: https://github.com/near/near-sdk-rs/pull/150

### Contract changes

* Fungible token now requires from the users to transfer NEAR tokens to pay for the storage increase to prevent the contract from locking the users from operating on it. https://github.com/near/near-sdk-rs/pull/173
* Renaming method of fungible token `set_allowance` => `inc_allowance`. Added `dec_allowance` method. https://github.com/near/near-sdk-rs/pull/174
* Remove possibility to do self-transfer in fungible token. https://github.com/near/near-sdk-rs/pull/176
* Improving fungible token comments https://github.com/near/near-sdk-rs/pull/177
* Add account check to `get_balance` in fungible token https://github.com/near/near-sdk-rs/pull/175
* In fungible token remove account from storage if its balance is 0 https://github.com/near/near-sdk-rs/pull/179
