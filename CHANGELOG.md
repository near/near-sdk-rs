# Changelog

## [Unreleased]

### Changed
- Updated `nearcore` crates used for unit testing to version `0.13.0`. [PR 820](https://github.com/near/near-sdk-rs/pull/820)
  - Removed `outcome` function from `MockedBlockchain` (incomplete and misleading data)
  - Changed `created_receipts` to return owned `Vec` instead of reference to one
  - `receipt_indices` field removed from `Receipt` type in testing utils
- Deprecate and remove `near-sdk-sim`. Removes `sim` proxy struct from `#[near_bindgen]`. [PR 817](https://github.com/near/near-sdk-rs/pull/817)
  - If `near-sdk-sim` tests can't be migrated to [workspaces-rs](https://github.com/near/workspaces-rs), `4.0.0-pre.9` version of `near-sdk-rs` and `near-sdk-sim` should be used

## [4.0.0-pre.9] - 2022-05-12

### Fixes
- near-contract-standards: `nft_tokens` in enumeration standard no longer panics when there are no tokens [PR 798](https://github.com/near/near-sdk-rs/pull/798)
- Optimized `nth` operation for `UnorderedMap` iterator and implemented `IntoIterator` for it. [PR 801](https://github.com/near/near-sdk-rs/pull/801)
  - This optimizes the `skip` operation, which is common with pagination

## [4.0.0-pre.8] - 2022-04-19

### Added
- Added `Debug` and `PartialEq` implementations for `PromiseError`. [PR 728](https://github.com/near/near-sdk-rs/pull/728).
- Added convenience function `env::block_timestamp_ms` to return ms since 1970. [PR 736](https://github.com/near/near-sdk-rs/pull/736)
- Added an optional way to handle contract errors with `Result`. [PR 745](https://github.com/near/near-sdk-rs/pull/745), [PR 754](https://github.com/near/near-sdk-rs/pull/754) and [PR 757](https://github.com/near/near-sdk-rs/pull/757).
- Added support for using `#[callback_result]` with a function that doesn't have a return. [PR 738](https://github.com/near/near-sdk-rs/pull/738)
- Support for multi-architecture docker builds and updated Rust version to 1.56 with latest [contract builder](https://hub.docker.com/r/nearprotocol/contract-builder). [PR 751](https://github.com/near/near-sdk-rs/pull/751)

### Fixes
- Disallow invalid `Promise::then` chains. Will now panic with `promise_1.then(promise_2.then(promise_3))` syntax. [PR 410](https://github.com/near/near-sdk-rs/pull/410)
  - Current implementation will schedule these promises in the incorrect order. With this format, it's unclear where the result from `promise_1` will be used, so it will panic at runtime.
- Fixed `signer_account_pk` from mocked implementation. [PR 785](https://github.com/near/near-sdk-rs/pull/785)

### Changed
- Deprecate `callback`, `callback_vec`, `result_serializer`, `init` proc macro attributes and remove exports from `near-sdk`. [PR 770](https://github.com/near/near-sdk-rs/pull/770)
  - They are not needed to be imported and are handled specifically within `#[near_bindgen]`
- Fixed gas assertion in `*_transfer_call` implementations of FT and NFT standards to only require what's needed. [PR 760](https://github.com/near/near-sdk-rs/pull/760)
- Fixed events being emitted in FT standard to include refund transfers and burn events. [PR 752](https://github.com/near/near-sdk-rs/pull/752)
- Moved `VMContext` to a local type defined in SDK to avoid duplicate types. [PR 785](https://github.com/near/near-sdk-rs/pull/785)
- Moved `Metadata` and `MethodMetadata` to a pseudo-private module as these are just types used within macros and not stable. [PR 771](https://github.com/near/near-sdk-rs/pull/771)

### Removed
- Remove `Clone` implementation for `Promise` (error prone) https://github.com/near/near-sdk-rs/pull/783

## [4.0.0-pre.7] - 2022-02-02

### Features
- Added FT and NFT event logs to `near-contract-standards`. [PR 627](https://github.com/near/near-sdk-rs/pull/627) and [PR 723](https://github.com/near/near-sdk-rs/pull/723)

## [4.0.0-pre.6] - 2022-01-21

### Features
- Added `env::random_seed_array` to return a fixed length array of the `random_seed` and optimizes the existing function. [PR 692](https://github.com/near/near-sdk-rs/pull/692)
- Implemented new iteration of `UnorderedSet` and `TreeMap` under `near_sdk::store` which is available with the `unstable` feature flag. [PR 672](https://github.com/near/near-sdk-rs/pull/672) and [PR 665](https://github.com/near/near-sdk-rs/pull/665)

### Fixes
- Improved macro spans for better errors with `#[near_bindgen]` macro. [PR 683](https://github.com/near/near-sdk-rs/pull/683)

## [4.0.0-pre.5] - 2021-12-23
- fix(standards): Fix NFT impl macros to not import `HashMap` and `near_sdk::json_types::U128`. [PR 571](https://github.com/near/near-sdk-rs/pull/571).
- Add drain iterator for `near_sdk::store::UnorderedMap`. [PR 613](https://github.com/near/near-sdk-rs/pull/613).
  - Will remove all values and iterate over owned values that were removed
- Fix codegen for methods inside a `#[near_bindgen]` to allow using `mut self` which will generate the same code as `self` and will not persist state. [PR 616](https://github.com/near/near-sdk-rs/pull/616).
- Make function call terminology consistent by switching from method name usages. [PR 633](https://github.com/near/near-sdk-rs/pull/633).
  - This is only a breaking change if inspecting the `VmAction`s of receipts in mocked environments. All other changes are positional argument names.
- Implement new iterator for `collections::Vec` to optimize for `nth` and `count`. [PR 634](https://github.com/near/near-sdk-rs/pull/634)
  - This is useful specifically for things like pagination, where `.skip(x)` will not load the first `x` elements anymore
  - Does not affect any `store` collections, which are already optimized, this just optimizes the legacy `collections` that use `Vec`
- Add consts for near, yocto, and tgas. [PR 640](https://github.com/near/near-sdk-rs/pull/640).
  - `near_sdk::ONE_NEAR`, `near_sdk::ONE_YOCTO`, `near_sdk::Gas::ONE_TERA`
- Update SDK dependencies for `nearcore` crates used for mocking (`0.10`) and `borsh` (`0.9`)
- Implemented `Debug` for all `collection` and `store` types. [PR 647](https://github.com/near/near-sdk-rs/pull/647)
- Added new internal mint function to allow specifying or ignoring refund. [PR 618](https://github.com/near/near-sdk-rs/pull/618)
- store: Implement caching `LookupSet` type. This is the new iteration of the previous version of `near_sdk::collections::LookupSet` that has an updated API, and is located at `near_sdk::store::LookupSet`. [PR 654](https://github.com/near/near-sdk-rs/pull/654), [PR 664](https://github.com/near/near-sdk-rs/pull/664).
- Deprecate `testing_env_with_promise_results`, `setup_with_config`, and `setup` due to these functions being unneeded anymore or have unintended side effects [PR 671](https://github.com/near/near-sdk-rs/pull/671)
  - Added missing pattern for only including context and vm config to `testing_env!` to remove friction
- Added `_array` suffix versions of `sha256`, `keccak256`, and `keccak512` hash functions in `env` [PR 646](https://github.com/near/near-sdk-rs/pull/646)
  - These return a fixed length array instead of heap allocating with `Vec<u8>`
- Added `ripemd160_array` hash function that returns a fixed length byte array [PR 648](https://github.com/near/near-sdk-rs/pull/648)
- Added `ecrecover` under `unstable` feature for recovering signer address by message hash and a corresponding signature. [PR 658](https://github.com/near/near-sdk-rs/pull/658).
- standards: Add require statement to ensure minimum needed gas in FT and NFT transfers at start of method. [PR 678](https://github.com/near/near-sdk-rs/pull/678)

## [4.0.0-pre.4] - 2021-10-15
- Unpin `syn` dependency in macros from `=1.0.57` to be more composable with other crates. [PR 605](https://github.com/near/near-sdk-rs/pull/605)

## [4.0.0-pre.3] - 2021-10-12
- Introduce `#[callback_result]` annotation, which acts like `#[callback]` except that it returns `Result<T, PromiseError>` to allow error handling. [PR 554](https://github.com/near/near-sdk-rs/pull/554)
  - Adds `#[callback_unwrap]` to replace `callback`
- mock: Update `method_names` field of `AddKeyWithFunctionCall` to a `Vec<String>` from `Vec<Vec<u8>>`. [PR 555](https://github.com/near/near-sdk-rs/pull/555)
  - Method names were changed to be strings in `4.0.0-pre.2` but this one was missed
- env: Update the register used for temporary `env` methods to `u64::MAX - 2` from `0`. [PR 557](https://github.com/near/near-sdk-rs/pull/557).
  - When mixing using `sys` and `env`, reduces chance of collision for using `0`
- store: Implement caching `LookupMap` type. This is the new iteration of the previous version of `near_sdk::collections::LookupMap` that has an updated API, and is located at `near_sdk::store::LookupMap`. [PR 487](https://github.com/near/near-sdk-rs/pull/487).
  - The internal storage format has changed from `collections::LookupMap` so the type cannot be swapped out without some migration.
- Implement `drain` iterator for `near_sdk::store::Vector`. [PR 592](https://github.com/near/near-sdk-rs/pull/592)
  - This allows any range of the vector to be removed and iterate on the removed values and the vector will be collapsed
- store: Implement caching `UnorderedMap` type. [PR 584](https://github.com/near/near-sdk-rs/pull/584).
  - Similar change to `LookupMap` update, and is an iterable version of that data structure.
  - Data structure has also changed internal storage format and cannot be swapped with `collections::UnorderedMap` without manual migration.

## [4.0.0-pre.2] - 2021-08-19
- Update `panic` and `panic_utf8` syscall signatures to indicate they do not return. [PR 489](https://github.com/near/near-sdk-rs/pull/489)
- Deprecate `env::panic` in favor of `env::panic_str`. [PR 492](https://github.com/near/near-sdk-rs/pull/492)
  - This method now takes a `&str` as the bytes are enforced to be utf8 in the runtime.
  - Change is symmetric to `env::log_str` change in `4.0.0-pre.1`
- Removes `PublicKey` generic on `env` promise batch calls. Functions now just take a reference to the `PublicKey`. [PR 495](https://github.com/near/near-sdk-rs/pull/495)
- fix: Public keys can no longer be borsh deserialized from invalid bytes. [PR 502](https://github.com/near/near-sdk-rs/pull/502)
  - Adds `Hash` derive to `PublicKey`
- Changes method name parameters from bytes (`Vec<u8>` and `&[u8]`) to string equivalents for batch function call promises [PR 515](https://github.com/near/near-sdk-rs/pull/515)
  - `promise_batch_action_function_call`, `Promise::function_call`, `promise_batch_action_add_key_with_function_call`, `Promise::add_access_key`, and `Promise::add_access_key_with_nonce` are afffected.
  - Updates `promise_then`, `promise_create`, and `Receipt::FunctionCall`'s method name to string equivalents from bytes [PR 521](https://github.com/near/near-sdk-rs/pull/521/files).
  - Instead of `b"method_name"` just use `"method_name"`, the bytes are enforced to be utf8 in the runtime.
- Fixes `#[ext_contract]` codegen function signatures to take an `AccountId` instead of a generic `ToString` and converting unchecked to `AccountId`. [PR 518](https://github.com/near/near-sdk-rs/pull/518)
- Fixes NFT contract standard `mint` function to not be in the `NonFungibleTokenCore` trait. [PR 525](https://github.com/near/near-sdk-rs/pull/525)
  - If using the `mint` function from the code generated function on the contract, switch to call it on the `NonFungibleToken` field of the contract (`self.mint(..)` => `self.token.mint(..)`)
- Fixes `nft_is_approved` method on contract standard to take `&self` instead of moving `self`.
- Fixes `receiver_id` in `mock::Receipt` to `AccountId` from string. This is a change to the type added in `4.0.0-pre.1`. [PR 529](https://github.com/near/near-sdk-rs/pull/529)
- Moves runtime syscalls to `near-sys` crate and includes new functions available [PR 507](https://github.com/near/near-sdk-rs/pull/507)

## [4.0.0-pre.1] - 2021-07-23
* Implements new `LazyOption` type under `unstable` feature. Similar to `Lazy` but is optional to set a value. [PR 444](https://github.com/near/near-sdk-rs/pull/444).
* Move type aliases and core types to near-sdk to avoid coupling. [PR 415](https://github.com/near/near-sdk-rs/pull/415).
* Implements new `Lazy` type under the new `unstable` feature which is a lazily loaded storage value. [PR 409](https://github.com/near/near-sdk-rs/pull/409).
* fix(promise): `PromiseOrValue` now correctly sets `should_return` flag correctly on serialization. [PR 407](https://github.com/near/near-sdk-rs/pull/407).
* fix(tree_map): Correctly panic when range indices are exluded and `start > end`. [PR 392](https://github.com/near/near-sdk-rs/pull/392).
* Implement `FromStr` for json types to allow calling `.parse()` to convert them.
  * `ValidAccountId` [PR 391](https://github.com/near/near-sdk-rs/pull/391).
  * `Base58CryptoHash` [PR 398](https://github.com/near/near-sdk-rs/pull/398).
  * `Base58PublicKey` [PR 400](https://github.com/near/near-sdk-rs/pull/400).
* expose `cur_block` and `genesis_config` from `RuntimeStandalone` to configure simulation tests. [PR 390](https://github.com/near/near-sdk-rs/pull/390).
* fix(simulator): failing with long chains. [PR 385](https://github.com/near/near-sdk-rs/pull/385).
* Make block time configurable to sim contract tests. [PR 378](https://github.com/near/near-sdk-rs/pull/378).
* Deprecate `env::log` in favour of `env::log_str`. The logs assume that the bytes are utf8, so this will be a cleaner interface to use. [PR 366](https://github.com/near/near-sdk-rs/pull/366).
* Update syscall interface to no longer go through `BLOCKCHAIN_INTERFACE`. Instead uses `near_sdk::sys` which is under the `unstable` feature flag if needed. [PR 417](https://github.com/near/near-sdk-rs/pull/417).
* Set up global allocator by default for WASM architectures. [PR 429](https://github.com/near/near-sdk-rs/pull/429).
  * This removes the re-export of `wee_alloc` because if this feature is enabled, the allocator will already be set.
  * Deprecates `setup_alloc!` macro as this will be setup by default, as long as the `wee_alloc` feature is not specifically disabled. In this case, the allocator can be overriden to a custom one or set manually if intended.
* Update `TreeMap` iterator implementation to avoid unnecessary storage reads. [PR 428](https://github.com/near/near-sdk-rs/pull/428).
* Update `AccountId` to be a newtype with merged functionality from `ValidAccountId`. [PR 448](https://github.com/near/near-sdk-rs/pull/448)
  * Removes `ValidAccountId` to avoid having multiple types for account IDs.
  * This type will have `ValidAccountId`'s JSON (de)serialization and the borsh serialization will be equivalent to what it was previously
* Initializes default for `BLOCKCHAIN_INTERFACE` to avoid requiring to initialize testing environment for tests that don't require custom blockchain interface configuration. [PR 450](https://github.com/near/near-sdk-rs/pull/450)
  * This default only affects outside of `wasm32` environments and is optional/backwards compatible
* Deprecates `env::block_index` and replaces it with `env::block_height` for more consistent naming. [PR 474](https://github.com/near/near-sdk-rs/pull/474)
* Updates internal NFT traits to not move the underlying type for methods. [PR 475](https://github.com/near/near-sdk-rs/pull/475)
  * This should not be a breaking change if using the `impl` macros, only if implementing manually
* Makes `BLOCKCHAIN_INTERFACE` a concrete type and no longer exports it. [PR 451](https://github.com/near/near-sdk-rs/pull/451)
  * If for testing you need this mocked blockchain, `near_sdk::mock::with_mocked_blockchain` can be used
  * `near_sdk::env::take_blockchain_interface` is removed, as this interface is no longer optional
  * removes `BlockchainInterface` trait, as this interface is only used in mocked contexts now
* Updates `Gas` type to be a newtype, which makes the API harder to misuse. [PR 471](https://github.com/near/near-sdk-rs/pull/471)
  * This also changes the JSON serialization of this type to a string, to avoid precision loss when deserializing in JavaScript
* `PublicKey` now utilizes `Base58PublicKey` instead of `Vec<u8>` directly [PR 453](https://github.com/near/near-sdk-rs/pull/453). Usage of `Base58PublicKey` is deprecated
* Expose `Receipt` and respective `VmAction`s in mocked contexts through replacing with a local interface and types. [PR 479](https://github.com/near/near-sdk-rs/pull/479)

## [3.1.0] - 2021-04-06

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

## [3.0.1] - 2021-03-25

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

## [2.0.1] - 2021-01-13

* Pinned version of `syn` crate to `=1.0.57`, since `1.0.58` introduced a breaking API change.

## [2.0.0] - 2020-08-25

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

## [1.0.1] - 2020-08-22

### Other changes

* Remove requirements for input args types to implement `serde::Serialize` and for return types to implement `serde::Deserialize`.

### Fix

* Bumped dependency version of `near-vm-logic` and `near-runtime-fees` to `2.0.0` that changed `VMLogic` interface.

## [1.0.0] - 2020-07-13

### Other changes

* Re-export common crates to be reused directly from `near_sdk`.
* Added `ValidAccountId` to `json_types` which validates the input string during deserialization to be a valid account ID.
* Added `Debug` to `Base58PublicKey`.
* Bumped dependency version of `borsh` to `0.7.0`.
* Bumped dependency version of `near-vm-logic` and `near-runtime-fees` to `1.0.0`.
* Implemented Debug trait for Vector collection that can be enabled with `expensive-debug` feature.

### Contract changes

* Use re-exported crate dependencies through `near_sdk` crate.

## [0.11.0] - 2020-06-08

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

[Unreleased]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.9...HEAD
[4.0.0-pre.9] https://github.com/near/near-sdk-rs/compare/4.0.0-pre.8...4.0.0-pre.9
[4.0.0-pre.8] https://github.com/near/near-sdk-rs/compare/4.0.0-pre.7...4.0.0-pre.8
[4.0.0-pre.7]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.6...4.0.0-pre.7
[4.0.0-pre.6]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.5...4.0.0-pre.6
[4.0.0-pre.5]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.4...4.0.0-pre.5
[4.0.0-pre.4]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.3...4.0.0-pre.4
[4.0.0-pre.3]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.2...4.0.0-pre.3
[4.0.0-pre.2]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.1...4.0.0-pre.2
[4.0.0-pre.1]: https://github.com/near/near-sdk-rs/compare/3.1.0...4.0.0-pre.1
[3.1.0]: https://github.com/near/near-sdk-rs/compare/3.0.1...3.1.0
[3.0.1]: https://github.com/near/near-sdk-rs/compare/v2.0.1...v3.0.1
[2.0.1]: https://github.com/near/near-sdk-rs/compare/v1.0.0...v2.0.1
[1.0.0]: https://github.com/near/near-sdk-rs/compare/v0.11.0...v1.0.0
[0.11.0]: https://github.com/near/near-sdk-rs/releases/tag/v0.11.0
