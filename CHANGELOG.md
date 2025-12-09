# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [5.22.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.21.0...near-sdk-v5.22.0) - 2025-12-09

### Added

- Added optional `arbitrary` feature (useful for fuzz testing) ([#1437](https://github.com/near/near-sdk-rs/pull/1437))

## [5.21.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.20.1...near-sdk-v5.21.0) - 2025-12-09

### Added

- Cache host funcs to save gas on repetitive calls (calling host is more expensive than caching inside Wasm) ([#1431](https://github.com/near/near-sdk-rs/pull/1431))

### Other

- Corrected VMContext.input doc comment to describe raw bytes ([#1435](https://github.com/near/near-sdk-rs/pull/1435))
- correct LookupSet docs to refer to set elements ([#1433](https://github.com/near/near-sdk-rs/pull/1433))
- Avoid cloning self_occurrences vectors in AttrSigInfo::new ([#1436](https://github.com/near/near-sdk-rs/pull/1436))
- Added tests for #[init] + #[private] macro combination ([#1432](https://github.com/near/near-sdk-rs/pull/1432))

## [5.20.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.20.0...near-sdk-v5.20.1) - 2025-12-05

### Other

- Fixed the doc-string in LookupSet to compare against UnorderedSet instead of itself ([#1428](https://github.com/near/near-sdk-rs/pull/1428))

## [5.20.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.19.0...near-sdk-v5.20.0) - 2025-12-03

### Added

- Re-export `near-account-id` crate as `near_sdk::account_id` ([#1426](https://github.com/near/near-sdk-rs/pull/1426))
- Introduced `StateInit` functions, clean up of the code ([#1425](https://github.com/near/near-sdk-rs/pull/1425))

### Other

- *(promise)* fix low-level links for add_access_key_allowance* ([#1427](https://github.com/near/near-sdk-rs/pull/1427))
- Relaxed iterator bounds to BorshDeserialize for UnorderedMap ([#1419](https://github.com/near/near-sdk-rs/pull/1419))

## [5.19.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.18.1...near-sdk-v5.19.0) - 2025-12-01

### Added

- Added support for refund_to and current_contract_code, updated deps to match nearcore 2.10 release ([#1423](https://github.com/near/near-sdk-rs/pull/1423))

### Other

- Clarified how keys are constructed for persistent NEAR SDK collections ([#1417](https://github.com/near/near-sdk-rs/pull/1417))

## [5.18.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.18.0...near-sdk-v5.18.1) - 2025-11-28

### Fixed

- Fixed docs.rs compilation error ([#1415](https://github.com/near/near-sdk-rs/pull/1415))

## [5.18.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.17.2...near-sdk-v5.18.0) - 2025-11-27

### Added

- relax owned String requirements ([#1404](https://github.com/near/near-sdk-rs/pull/1404))
- Add Event::to_event_log() utility function ([#1394](https://github.com/near/near-sdk-rs/pull/1394))
- be explicit about detached `Promise`s ([#1400](https://github.com/near/near-sdk-rs/pull/1400))
- *(near-sdk-macros)* `#[near(contract_state(key = b"CUSTOM"))]` ([#1399](https://github.com/near/near-sdk-rs/pull/1399))
- optimize `Promise::and` ([#1396](https://github.com/near/near-sdk-rs/pull/1396))
- use #[serde_as] for #[near(serializers = [json])] ([#1393](https://github.com/near/near-sdk-rs/pull/1393))
- Introduce new method `::ext_on(promise)` for all Ext Contract Traits for using high-level APIs for batching actions into a single promise receipt ([#1413](https://github.com/near/near-sdk-rs/pull/1413))

### Fixed

- Pass mutable buffers to sys out-params in balance and stake getters ([#1412](https://github.com/near/near-sdk-rs/pull/1412))
- Fixed the `TreeMap::range()` method to respect lower Bound::Unbounded ([#1408](https://github.com/near/near-sdk-rs/pull/1408))
- *(serde)* avoid String allocation in error mapping for integers and hash ([#1411](https://github.com/near/near-sdk-rs/pull/1411))
- Fix serde_as and ordering of the fields in AsNep297Event ([#1405](https://github.com/near/near-sdk-rs/pull/1405))
- Added support for #[private] attribute for #[init] methods ([#1410](https://github.com/near/near-sdk-rs/pull/1410))
- allow PanicOnDefault on eums ([#1401](https://github.com/near/near-sdk-rs/pull/1401))

### Other

- Use CryptoHash wherever applicable and other small papercut fixes ([#1387](https://github.com/near/near-sdk-rs/pull/1387))
- fix example header format for readme.md ([#1407](https://github.com/near/near-sdk-rs/pull/1407))

## [5.17.2](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.17.1...near-sdk-v5.17.2) - 2025-08-30

### Other

- Auto-document features on docs.rs + improved wording for the top-level documentation ([#1386](https://github.com/near/near-sdk-rs/pull/1386))

## [5.17.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.17.0...near-sdk-v5.17.1) - 2025-08-19

### Other

- Enabled global-contracts feature for docs.rs ([#1383](https://github.com/near/near-sdk-rs/pull/1383))

## [5.17.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.16.0...near-sdk-v5.17.0) - 2025-08-19

### Added

- Add global contract support (NEP-591) ([#1369](https://github.com/near/near-sdk-rs/pull/1369))

## [5.16.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.15.1...near-sdk-v5.16.0) - 2025-08-15

### Added

- `then_concurrent()` + `join()` Promise API for map-reduce-like flow ([#1364](https://github.com/near/near-sdk-rs/pull/1364))

### Other

- [**breaking** for unit-testing feature] update near-* dependencies to 0.31 release ([#1379](https://github.com/near/near-sdk-rs/pull/1379))

## [5.15.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.15.0...near-sdk-v5.15.1) - 2025-07-01

### Other

- New `non-contract-usage` feature flag to be able to use near-sdk-rs in thirdparty projects that don't use it for contract building ([#1370](https://github.com/near/near-sdk-rs/pull/1370))

## [5.15.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.14.0...near-sdk-v5.15.0) - 2025-06-16

### Added

- Added a new `.to_json()` method for Events in addition to `.emit()` ([#1360](https://github.com/near/near-sdk-rs/pull/1360))
- Hint developers to use `cargo near build` to build contracts instead of `cargo build` ([#1361](https://github.com/near/near-sdk-rs/pull/1361))
- Include detailed error information on deserialization errors for function input arguments to improve troubleshooting experience for devs ([#1363](https://github.com/near/near-sdk-rs/pull/1363))

### Other

- expand cfgs' compilation error with complete reason of the error ([#1367](https://github.com/near/near-sdk-rs/pull/1367))
- Added "How to Deploy a Smart Contract on NEAR | Full Guide for Windows, Mac & Linux (Step-by-Step)" video to the README ([#1366](https://github.com/near/near-sdk-rs/pull/1366))
- Added explanation for borsh(...) parameters in #[near(serializers = [...])] ([#1359](https://github.com/near/near-sdk-rs/pull/1359))

## [5.14.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.13.0...near-sdk-v5.14.0) - 2025-05-14

### Other

- updates near-workspaces to 0.20 version ([#1358](https://github.com/near/near-sdk-rs/pull/1358))
- updates near-* dependencies to 0.30 release ([#1356](https://github.com/near/near-sdk-rs/pull/1356))

## [5.13.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.12.0...near-sdk-v5.13.0) - 2025-05-05

### Added

- Added BLS12-381 curve operations support in near-sys and exposed in near_sdk::env ([#1346](https://github.com/near/near-sdk-rs/pull/1346))

### Fixed

- BLS12-381 pairing check return value comparison ([#1352](https://github.com/near/near-sdk-rs/pull/1352))
- Fixed the tokenization of of ContractMetadata, where it can lead to invalid meta if `link` and `version` is not provided ([#1349](https://github.com/near/near-sdk-rs/pull/1349))

### Other

- update `near-workspaces`, `cargo-near-build`in examples and core tests  to avoid `wasm-opt` compile ([#1350](https://github.com/near/near-sdk-rs/pull/1350))

## [5.12.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.11.0...near-sdk-v5.12.0) - 2025-04-10

### Added

- nep330 result wasm path field  ([#1344](https://github.com/near/near-sdk-rs/pull/1344))

### Other

- Fixed linting warnings (clippy Rust 1.86) ([#1345](https://github.com/near/near-sdk-rs/pull/1345))
- added example how to use `promise_yield_create` and `promise_yield_resume` ([#1133](https://github.com/near/near-sdk-rs/pull/1133))
- updated near-workspaces in examples ([#1337](https://github.com/near/near-sdk-rs/pull/1337))

## [5.11.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.10.0...near-sdk-v5.11.0) - 2025-03-22

### Added

- Added promise_yield_create and promise_yield_resume APIs to the mocked test utils ([#1333](https://github.com/near/near-sdk-rs/pull/1333))

## [5.10.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.9.0...near-sdk-v5.10.0) - 2025-03-21

### Added

- `deny_unknown_arguments` on-method attribute ([#1328](https://github.com/near/near-sdk-rs/pull/1328))

## [5.9.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.8.2...near-sdk-v5.9.0) - 2025-03-06

### Other

- updates near-* dependencies to 0.29 release ([#1325](https://github.com/near/near-sdk-rs/pull/1325))

## [5.8.2](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.8.1...near-sdk-v5.8.2) - 2025-03-06

### Other

- impl details of `#[callback_unwrap]` ([#1321](https://github.com/near/near-sdk-rs/pull/1321))
- `#[near(contract_state)]` in-depth pass ([#1307](https://github.com/near/near-sdk-rs/pull/1307))
- document `callback_unwrap` attribute ([#1313](https://github.com/near/near-sdk-rs/pull/1313))

## [5.8.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.8.0...near-sdk-v5.8.1) - 2025-02-17

### Other

- remove `double_contract_state_error` diagnostic (#1310)

## [5.8.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.7.1...near-sdk-v5.8.0) - 2025-02-07

### Added

- *(near_sdk_macros)* improved error reporting for `near` macro (#1301)

### Other

- moved annotations to the near macro documentation.  (#1299)
- moved near-sdk-macros doc to near-sdk crate. (#1295)

## [5.7.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.7.0...near-sdk-v5.7.1) - 2025-01-30

### Other

- improved documentation for near-sdk and near-sdk-macros crates (#1262)
- clippy lint of 1.84 fixed (#1290)
- `__abi-generate` feature in docs.rs (#1286)
- updates near-workspaces to 0.16 version (#1284)
- impaired PublicKey with missing BorshSchema (#1281)

## [5.7.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.6.0...near-sdk-v5.7.0) - 2024-12-13

### Other

- updates near-* dependencies to 0.28 release (#1272)
- tests for Lazy and moving out of unstable (#1268)
- add a `cargo doc` job (#1269)
- allow clippy::needless_lifetimes (1.83 more suggestions) (#1267)
- examples for Near-related host functions (#1259)
- updates near-workspaces to 0.15 version (#1260)

## [5.6.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.5.0...near-sdk-v5.6.0) - 2024-11-14

### Other

- updates near-* dependencies to 0.27 release ([#1254](https://github.com/near/near-sdk-rs/pull/1254))
- freeze 1.81 for near-workspaces paths (temporarily) ([#1250](https://github.com/near/near-sdk-rs/pull/1250))
- Benchmark near collections and provide the results as the reference in the docs ([#1248](https://github.com/near/near-sdk-rs/pull/1248))
- Updated near-workspaces to 0.14 version (matching 0.26 near-* release) ([#1246](https://github.com/near/near-sdk-rs/pull/1246))

## [5.5.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.4.0...near-sdk-v5.5.0) - 2024-09-11

### Other

- Updated near-* dependendencies to v0.26.0. Migrated testing blockchain mock to C-unwind ([#1244](https://github.com/near/near-sdk-rs/pull/1244))

## [5.4.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.3.0...near-sdk-v5.4.0) - 2024-09-04

### Other
- updates near-* dependencies to 0.25.0 ([#1242](https://github.com/near/near-sdk-rs/pull/1242))
- updates near-workspaces-rs ([#1239](https://github.com/near/near-sdk-rs/pull/1239))

## [5.3.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.2.1...near-sdk-v5.3.0) - 2024-08-13

### Added
- Introduced 'remove' method for 'near_sdk::store::Lazy' collection ([#1238](https://github.com/near/near-sdk-rs/pull/1238))
- Allow store collection iterators to be cloned (this enables standard Iterator methods like `cycle()`) ([#1224](https://github.com/near/near-sdk-rs/pull/1224))

### Fixed
- Fix storage management error message with proper amount ([#1222](https://github.com/near/near-sdk-rs/pull/1222))
- Fixed compilation errors after Rust 1.80 latest stable release ([#1227](https://github.com/near/near-sdk-rs/pull/1227))

### Other
- updates near-* dependencies to 0.24.0 ([#1237](https://github.com/near/near-sdk-rs/pull/1237))
- Include all examples into CI testing suite ([#1228](https://github.com/near/near-sdk-rs/pull/1228))
- Optimized up to 10% contract binary size by using `near_sdk::env::panic_str` instead of `expect` calls ([#1220](https://github.com/near/near-sdk-rs/pull/1220))
- Fixed Rust 1.80 new warning by adding `cargo:rustc-check-cfg` for `__abi-embed-checked` feature in `near-sdk-macros` build.rs ([#1225](https://github.com/near/near-sdk-rs/pull/1225))

## [5.2.1](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.2.0...near-sdk-v5.2.1) - 2024-07-05

### Fixed
- *(nep330)* Fallback to `CARGO_PKG_REPOSITORY` and `CARGO_PKG_VERSION` when `NEP330_*` variables are not provided ([#1215](https://github.com/near/near-sdk-rs/pull/1215))

## [5.2.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.1.0...near-sdk-v5.2.0) - 2024-07-04

### Added
- New `near_sdk::store::IterableMap` and `near_sdk::store::IterableSet` that address the iteration performance issue of `store::UnorderedMap` ([#1164](https://github.com/near/near-sdk-rs/pull/1164)) ([#1175](https://github.com/near/near-sdk-rs/pull/1175))
- Added `BorshSchema` trait impl to all `near_sdk::store` collections!
  - `store::TreeMap<K, V, H>` and `UnorderedSet<T, H>` ([#1213](https://github.com/near/near-sdk-rs/pull/1213))
  - `store::IterableSet` and `store::IterableMap` and refactored and added ABI defiintions tests ([#1212](https://github.com/near/near-sdk-rs/pull/1212))
  - `store::UnorderedMap` ([#1209](https://github.com/near/near-sdk-rs/pull/1209))
- Added yield execution host functions ([#1183](https://github.com/near/near-sdk-rs/pull/1183))
- NEP-330 1.2.0 support - added build info field in contract metadata ([#1178](https://github.com/near/near-sdk-rs/pull/1178))

### Fixed
- [**technically breaking**] Make log macro fully compatible with std::format (string interpolation is now supported) ([#1189](https://github.com/near/near-sdk-rs/pull/1189))
- use FQDNs when calling contract methods to avoid method names collision ([#1186](https://github.com/near/near-sdk-rs/pull/1186))

### Other
- Added performance tests for 'store' collections ([#1195](https://github.com/near/near-sdk-rs/pull/1195))
- Full tests coverage for `store::Vector` + coverage for all the collections relevant to IterableMap implementation ([#1173](https://github.com/near/near-sdk-rs/pull/1173))
- Full tests coverage for `store` collections ([#1172](https://github.com/near/near-sdk-rs/pull/1172))
- Documented `#[init]`, `#[payable]`, `#[handle_result]`, `#[private]`, `#[result_serializer]` attributes for docs.rs discoverability ([#1185](https://github.com/near/near-sdk-rs/pull/1185))
- Enabled `unit-testing` feature for docs.rs
- Replaced manual `borsh` trait impl-s with derives and correct bounds in `near_sdk::store` and `near_sdk::collections` ([#1176](https://github.com/near/near-sdk-rs/pull/1176))
- Proxy JsonSchema::schema_name to the original implementation ([#1210](https://github.com/near/near-sdk-rs/pull/1210))
- Fixed Rust 1.79 linter warnings ([#1202](https://github.com/near/near-sdk-rs/pull/1202))
- Fixed Rust 1.78 linter warnings ([#1181](https://github.com/near/near-sdk-rs/pull/1181))
- Updated near-* dependencies to 0.23 version ([#1207](https://github.com/near/near-sdk-rs/pull/1207))

## [5.1.0](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.0.0...near-sdk-v5.1.0) - 2024-03-28

### Added
- Finalize `#[near]` attribute-macro implementation with the support for custom parameters passing to serializer attributes `#[near(serializers = [borsh(...)])]` ([#1158](https://github.com/near/near-sdk-rs/pull/1158))
- Introduce `#[near]` macro to further streamline contracts development reducing the boilerplate! ([#1142](https://github.com/near/near-sdk-rs/pull/1142))

### Other
- add typo checker in ci ([#1159](https://github.com/near/near-sdk-rs/pull/1159))

## [5.0.0](https://github.com/near/near-sdk-rs/compare/4.1.1...near-sdk-v5.0.0) - 2024-02-21

### Highlights

This release mostly maintains backwards compatibility with the previous version, but it also includes several breaking changes that improve developer experience and bring security and performance fixes. The most notable changes are:

- Contract source metadata ([NEP-330](https://github.com/near/NEPs/blob/master/neps/nep-0330.md)) is now implemented by default for all the contracts out of the box, which means that you can call `contract_source_metadata()` function and receive `{ version?: string, link?: string, standards?: { standard: string, version: string }[] }` ([#1106](https://github.com/near/near-sdk-rs/pull/1106))
- Type-safe NEAR balance, gas amounts, and account ids were implemented:
  - Use [`near_sdk::NearToken`](https://docs.rs/near-sdk/5.0.0/near_sdk/struct.NearToken.html) instead of u128/U128/Balance ([#1104](https://github.com/near/near-sdk-rs/pull/1104))
  - Use [`near_sdk::Gas`](https://docs.rs/near-sdk/5.0.0/near_sdk/struct.Gas.html) instead of u64/Gas ([#1082](https://github.com/near/near-sdk-rs/pull/1082))
  - Use [`near_sdk::AccountId`](https://docs.rs/near-sdk/5.0.0/near_sdk/struct.AccountId.html) or [`near_sdk::AccountIdRef`](https://docs.rs/near-sdk/5.0.0/near_sdk/struct.AccountIdRef.html) instead of String aliases for account ids ([#1108](https://github.com/near/near-sdk-rs/pull/1108))
- Update [borsh to 1.0.0](https://github.com/near/borsh-rs/releases/tag/borsh-v1.0.0) ([#1075](https://github.com/near/near-sdk-rs/pull/1075))
  - You will have to be explicit about the borsh re-export with `#[borsh(crate = "near_sdk::borsh")]`, see the example in the [README](https://github.com/near/near-sdk-rs#example)
- New host functions exposed:
  - [`near_sdk::env::ed25519_verify`](https://docs.rs/near-sdk/5.0.0/near_sdk/env/fn.ed25519_verify.html) ([#1010](https://github.com/near/near-sdk-rs/pull/1010))
  - [`near_sdk::env::alt_bn128`](https://docs.rs/near-sdk/5.0.0/near_sdk/env/fn.alt_bn128.html) ([#1028](https://github.com/near/near-sdk-rs/pull/1028))
- Slimmed down the dependencies by default, most notably, you may still need to explicitly enable `legacy` feature for `near_sdk::collections` and `unit-testing` feature for `near_sdk::testing_env` and `near_sdk::mock` ([#1149](https://github.com/near/near-sdk-rs/pull/1149))
- Updated `nearcore` crates from `0.17` -> `0.20`, but contracts rarely use these directly so no breaking changes are expected ([#1130](https://github.com/near/near-sdk-rs/pull/1130))
- Support Result types in `#[handle_result]` regardless of how they're referred to ([#1099](https://github.com/near/near-sdk-rs/pull/1099))
- `Self` is now prohibited in non-init methods to prevent common footguns ([#1073](https://github.com/near/near-sdk-rs/pull/1073))
- Require explicit `Unlimited` or `Limited` when specifying allowances to prevent `0` to be silently treated as unlimited allowance ([#976](https://github.com/near/near-sdk-rs/pull/976))
- Performance improvement to `TreeMap.range` ([#964](https://github.com/near/near-sdk-rs/pull/964))
- Deprecated `near_sdk::store::UnorderedMap` and `near_sdk::store::UnorderedSet` due to not meeting the original requirements (iteration over a collection of more than 2k elements runs out of gas) ([#1139](https://github.com/near/near-sdk-rs/pull/1139))
- Deprecated `near_sdk::collections::LegacyTreeMap` ([#963](https://github.com/near/near-sdk-rs/pull/963))

The best way to develop NEAR contracts in Rust is by using [`cargo-near` CLI](https://github.com/near/cargo-near).
It provides a convenient way to create, build, test, and deploy contracts!

Get your fully configured development environment in under 1 minute using [GitHub CodeSpaces configured for NEAR](https://github.com/near/cargo-near-new-project-template)!

## [5.0.0-alpha.3](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.0.0-alpha.2...near-sdk-v5.0.0-alpha.3) - 2024-02-19

### Fixed
- Deprecated `store::UnorderedMap` and `store::UnorderedSet` due to not meeting the original requirements (iteration over a collection of more than 2k elements runs out of gas) ([#1139](https://github.com/near/near-sdk-rs/pull/1139))

### Other
- Added ABI tests for SDK-generated methods [contract_source_metadata] ([#1136](https://github.com/near/near-sdk-rs/pull/1136))

## [5.0.0-alpha.2](https://github.com/near/near-sdk-rs/compare/near-sdk-v5.0.0-alpha.1...near-sdk-v5.0.0-alpha.2) - 2024-01-16

### Fixed
- include `near-sdk/src/private/result_type_ext.rs` file into module tree ([#1122](https://github.com/near/near-sdk-rs/pull/1122))
- Fixed `contract_source_metadata` compilation issue when multiple impl blocks are there ([#1118](https://github.com/near/near-sdk-rs/pull/1118))
- remove leftover `near_sdk::__private::schemars` ([#1120](https://github.com/near/near-sdk-rs/pull/1120))

### Other
- [**breaking**] update `nearcore` crates from `0.17` -> `0.20` ([#1130](https://github.com/near/near-sdk-rs/pull/1130))
- fix new 1.75 warnings ([#1128](https://github.com/near/near-sdk-rs/pull/1128))
- Re-exported packages cleanup ([#1114](https://github.com/near/near-sdk-rs/pull/1114))

## [5.0.0-alpha.1](https://github.com/near/near-sdk-rs/compare/4.1.1...near-sdk-v5.0.0-alpha.1) - 2023-11-18

### Added
- adding `nep-0330` contract source metadata info ([#1106](https://github.com/near/near-sdk-rs/pull/1106))
- Support Result types in #[handle_result] regardless of how they're referred to ([#1099](https://github.com/near/near-sdk-rs/pull/1099))
- accumulate compilation errors to provide them all at once ([#1097](https://github.com/near/near-sdk-rs/pull/1097))
- [**breaking**] prohibit `Self` in non-init methods to prevent common footguns ([#1073](https://github.com/near/near-sdk-rs/pull/1073))
- [**breaking**] Make `PromiseIndex` a newtype, so it cannot be misused ([#1066](https://github.com/near/near-sdk-rs/pull/1066))
- Expose alt_bn128 curve host functions via near_sdk::env ([#1028](https://github.com/near/near-sdk-rs/pull/1028))
- Deprecate `Self` in non-init function's return type ([#1030](https://github.com/near/near-sdk-rs/pull/1030))
- new `near_sdk::store::UnorderedMap::defrag` method useful for gas tuning ([#1023](https://github.com/near/near-sdk-rs/pull/1023))
- add ed25519_verify host function ([#1010](https://github.com/near/near-sdk-rs/pull/1010))
- add `Default` implementation to JSON types ([#1018](https://github.com/near/near-sdk-rs/pull/1018))

### Fixed
- Fixed compilation-tests after stable Rust release 1.72 ([#1081](https://github.com/near/near-sdk-rs/pull/1081))
- Un-deprecate private init functions ([#1074](https://github.com/near/near-sdk-rs/pull/1074))
- *(store::TreeMap)* remove of the entry API now correctly updates the tree root when changed ([#995](https://github.com/near/near-sdk-rs/pull/995))
- strip return types of lifetimes ([#982](https://github.com/near/near-sdk-rs/pull/982))
- prohibit NEAR function generics ([#980](https://github.com/near/near-sdk-rs/pull/980))
- concretize `Self` references in method signatures ([#1001](https://github.com/near/near-sdk-rs/pull/1001))
- make event `emit` public ([#975](https://github.com/near/near-sdk-rs/pull/975))
- Exposed missing iterator types used in `near_sdk::store::UnorderedSet` ([#961](https://github.com/near/near-sdk-rs/pull/961))
- add compiler error for using Result with init ([#1024](https://github.com/near/near-sdk-rs/pull/1024))
- fully qualify the schema_container method call ([#1003](https://github.com/near/near-sdk-rs/pull/1003))
- `__abi-embed` compilation error ([#971](https://github.com/near/near-sdk-rs/pull/971))

### Other
- [**breaking**] Use type-safe NearToken instead of u128/U128 ([#1104](https://github.com/near/near-sdk-rs/pull/1104))
- migrate to a external near-account-id crate for reusable AccountId type ([#1108](https://github.com/near/near-sdk-rs/pull/1108))
- [**breaking**] Delete the deprecated metadata module from near-sdk-macros in favor of near-abi ([#1098](https://github.com/near/near-sdk-rs/pull/1098))
- documented env::random_seed ([#1096](https://github.com/near/near-sdk-rs/pull/1096))
- Update borsh to 1.0.0 ([#1075](https://github.com/near/near-sdk-rs/pull/1075))
- bump version of near-workspaces ([#1094](https://github.com/near/near-sdk-rs/pull/1094))
- upgrade syn crate from version 1 to 2 ([#1088](https://github.com/near/near-sdk-rs/pull/1088))
- Move from Gas to NearGas from near-gas crate ([#1082](https://github.com/near/near-sdk-rs/pull/1082))
- Respect `{{ matrix.toolchain }}` in "Test Core: test" job ([#1085](https://github.com/near/near-sdk-rs/pull/1085))
- Add release-plz to automate releases ([#1069](https://github.com/near/near-sdk-rs/pull/1069))
- add `add_access_key` test coverage ([#1029](https://github.com/near/near-sdk-rs/pull/1029))
- disentangle bindgen extractor logic ([#1025](https://github.com/near/near-sdk-rs/pull/1025))
- Bumped supported rust version to minimum 1.68 - reflected in BuildKite ([#1014](https://github.com/near/near-sdk-rs/pull/1014))
- Update visibility of FreeList and method ([#998](https://github.com/near/near-sdk-rs/pull/998))
- Add documentation to collection cache types ([#997](https://github.com/near/near-sdk-rs/pull/997))
- abstract common functions in `Keys` and `KeysRange` ([#989](https://github.com/near/near-sdk-rs/pull/989))
- perf (`TreeMap.range`): Update the TreeMap->Range logic ([#964](https://github.com/near/near-sdk-rs/pull/964))
- Took out a footgun with allowances ([#976](https://github.com/near/near-sdk-rs/pull/976))
- Depreciated legacy tree map  ([#963](https://github.com/near/near-sdk-rs/pull/963))
- Removed the not ready enum type ([#977](https://github.com/near/near-sdk-rs/pull/977))
- use `insta` crate for testing macro generated code ([#1090](https://github.com/near/near-sdk-rs/pull/1090))
- Use global paths in macro expansions ([#1060](https://github.com/near/near-sdk-rs/pull/1060))
- fix typo ([#1052](https://github.com/near/near-sdk-rs/pull/1052))
- change private init method from error to warning ([#1043](https://github.com/near/near-sdk-rs/pull/1043))
- cover all features with clippy ([#1044](https://github.com/near/near-sdk-rs/pull/1044))
- use attr sig info in abi generator ([#1036](https://github.com/near/near-sdk-rs/pull/1036))
- disentangle bindgen code generation ([#1033](https://github.com/near/near-sdk-rs/pull/1033))

## [4.1.1] - 2022-11-10

### Fixed
- Fixed invalid import from "legacy" feature flag from stabilized collection. [PR 960](https://github.com/near/near-sdk-rs/pull/960)

### Removed
- Deprecated declarative macros for NFT impl code generation. [PR 1042](https://github.com/near/near-sdk-rs/pull/1042)
- Deprecated declarative macros for FT impl code generation. [PR 1054](https://github.com/near/near-sdk-rs/pull/1054)

## [4.1.0] - 2022-11-09

### Added
- Added `near_sdk::NearSchema` derive macro for convenience in implementing schema types for `abi`. [PR 891](https://github.com/near/near-sdk-rs/pull/891).
- Added support for custom events with `#[near_bindgen(event_json(standard = "___"))]` syntax. [PR 934](https://github.com/near/near-sdk-rs/pull/934)

### Changed
- Added new `legacy` feature flag and put `near_sdk::collections` under it. `near_sdk::store` will be replacing them. [PR 923](https://github.com/near/near-sdk-rs/pull/923).
- Stabilize `store::LookupMap` and `store::UnorderedMap` collections. [PR 922](https://github.com/near/near-sdk-rs/pull/922).
- Stabilize `store::LookupSet` and `store::UnorderedSet` collections. [PR 924](https://github.com/near/near-sdk-rs/pull/924).
- `abi` feature flag is now enabled by default. [PR 956](https://github.com/near/near-sdk-rs/pull/956).
- Updated `near-abi` version to `0.3.0`. [PR 954](https://github.com/near/near-sdk-rs/pull/954).

### Removed
- Deleted `metadata` macro. Use https://github.com/near/abi instead. [PR 920](https://github.com/near/near-sdk-rs/pull/920)
- Deprecated `ReceiptIndex` and `IteratorIndex` vm types. [PR 949](https://github.com/near/near-sdk-rs/pull/949).

### Fixes
- Updated the associated error type for `Base58CryptoHash` parsing through `TryFrom` to concrete type. [PR 919](https://github.com/near/near-sdk-rs/pull/919)

## [4.1.0-pre.3] - 2022-08-30

### Added
- Enabled ABI embedding in contract through `__abi-embed` feature and [cargo-near](https://github.com/near/cargo-near). [PR 893](https://github.com/near/near-sdk-rs/pull/893)
- Added `schemars::JsonSchema` implementations for `NFT` contract standard types to enable ABI generation. [PR 904](https://github.com/near/near-sdk-rs/pull/904)

### Changed
- Stabilized `store::Lazy` and `store::LazyOption` types and updated their debug implementations. [PR 897](https://github.com/near/near-sdk-rs/pull/897) [PR 888](https://github.com/near/near-sdk-rs/pull/888)

## [4.1.0-pre.2] - 2022-08-26

### Added
- Support newly stabilized `alt_bn128` host functions that were recently stabilized. [PR 885](https://github.com/near/near-sdk-rs/pull/885)
- Added `Eq` implementations for various types. [PR 887](https://github.com/near/near-sdk-rs/pull/887)
- `alt_bn128` host functions supported in testing utils. [PR 885](https://github.com/near/near-sdk-rs/pull/885)

### Fixes
- Standards: NFT storage estimation bug fix and fix retrieval requiring enum and enumeration standard implementation. [PR 843](https://github.com/near/near-sdk-rs/pull/843)

### Changed
- `near_sdk::store::Vector` stabilized. [PR 815](https://github.com/near/near-sdk-rs/pull/815)
- [ABI](https://github.com/near/abi) primitives moved into [near-abi-rs](https://github.com/near/near-abi-rs). [PR 889](https://github.com/near/near-sdk-rs/pull/889)

## [4.1.0-pre.1] - 2022-08-05

### Added
- Exposed Rustdocs to exposed ABI type. [PR 876](https://github.com/near/near-sdk-rs/pull/876)

### Changed
- Updated `nearcore` dependencies used for unit testing to `0.14`. [PR 875](https://github.com/near/near-sdk-rs/pull/875)

### Fixed
- Handling of certain types through ABI macros. [PR 877](https://github.com/near/near-sdk-rs/pull/877)

## [4.1.0-pre.0] - 2022-07-29

### Added
- `abi` feature to expose metadata about contract and functions to be consumed by [cargo-near](https://github.com/near/cargo-near). [PR 831](https://github.com/near/near-sdk-rs/pull/831), [PR 863](https://github.com/near/near-sdk-rs/pull/863), [PR 858](https://github.com/near/near-sdk-rs/pull/858)
- Exposed `ext_ft_metadata` to call `FungibleTokenMetadataProvider` trait from an external contract. [PR 836](https://github.com/near/near-sdk-rs/pull/836)

### Fixed
- Safe math fixes for fungible token standard. [PR 830](https://github.com/near/near-sdk-rs/pull/830)
  - This just ensures that there is no overflow if `overflow-checks` is not enabled by cargo

### Changed
- Enabled const-generics feature by default on borsh. [PR 828](https://github.com/near/near-sdk-rs/pull/828)
- License changed from GPL-3 to MIT or Apache. [PR 837](https://github.com/near/near-sdk-rs/pull/837)
- Put unit-testing logic behind `unit-testing` flag, which is enabled by default. [PR 870](https://github.com/near/near-sdk-rs/pull/870)
  - This pulls in `nearcore` dependencies to mock the VM, so can turn off default-features to compile faster

### Removed
- Deprecated `near_contract_standards::upgrade`. [PR 856](https://github.com/near/near-sdk-rs/pull/856)
  - Implementation did not match any NEAR standard and was not correct

## [4.0.0] - 2022-05-25

### Added
- Added `Eq`, `PartialOrd`, `Ord` to `json_types` integer types. [PR 823](https://github.com/near/near-sdk-rs/pull/823)

### Changed
- Updated cross-contract, `ext` API for new [`NEP264`](https://github.com/near/NEPs/pull/264) functionality. [PR 742](https://github.com/near/near-sdk-rs/pull/742)
  - More details on the API change can be found [here](https://github.com/near/near-sdk-rs/issues/740)
  - This API uses a default weight of `1` with no static gas, but this weight, the static gas, and the attached deposit can all be modified on any external call
  - `ext` methods are added to each `#[near_bindgen]` contract struct by default and for each method for convenience
- Updated `nearcore` crates used for unit testing to version `0.13.0`. [PR 820](https://github.com/near/near-sdk-rs/pull/820)
  - Removed `outcome` function from `MockedBlockchain` (incomplete and misleading data)
  - Changed `created_receipts` to return owned `Vec` instead of reference to one
  - `receipt_indices` field removed from `Receipt` type in testing utils
- Deprecate and remove `near-sdk-sim`. Removes `sim` proxy struct from `#[near_bindgen]`. [PR 817](https://github.com/near/near-sdk-rs/pull/817)
  - If `near-sdk-sim` tests can't be migrated to [workspaces-rs](https://github.com/near/workspaces-rs), `4.0.0-pre.9` version of `near-sdk-rs` and `near-sdk-sim` should be used
- Optimized read_register to read to non-zeroed buffer. [PR 804](https://github.com/near/near-sdk-rs/pull/804)
- Switched Rust edition for libraries to `2021`. [PR 669](https://github.com/near/near-sdk-rs/pull/669)

### Fixes
- Avoid loading result bytes with `near_sdk::is_promise_success()`. [PR 816](https://github.com/near/near-sdk-rs/pull/816)


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
* fix(tree_map): Correctly panic when range indices are excluded and `start > end`. [PR 392](https://github.com/near/near-sdk-rs/pull/392).
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
  * Deprecates `setup_alloc!` macro as this will be setup by default, as long as the `wee_alloc` feature is not specifically disabled. In this case, the allocator can be overridden to a custom one or set manually if intended.
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

[Unreleased]: https://github.com/near/near-sdk-rs/compare/4.1.1...HEAD
[4.1.1]: https://github.com/near/near-sdk-rs/compare/4.1.0...4.1.1
[4.1.0]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.3...4.1.0
[4.1.0-pre.3]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.2...4.1.0-pre.3
[4.1.0-pre.2]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.1...4.1.0-pre.2
[4.1.0-pre.1]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.0...4.1.0-pre.1
[4.1.0-pre.0]: https://github.com/near/near-sdk-rs/compare/4.0.0...4.1.0-pre.0
[4.0.0]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.9...4.0.0
[4.0.0-pre.9]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.8...4.0.0-pre.9
[4.0.0-pre.8]: https://github.com/near/near-sdk-rs/compare/4.0.0-pre.7...4.0.0-pre.8
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
