# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [5.5.0](https://github.com/near/near-sdk-rs/compare/near-contract-standards-v5.4.0...near-contract-standards-v5.5.0) - 2024-09-11

### Other

- Updated near-* dependendencies to v0.26.0. Migrated testing blockchain mock to C-unwind ([#1244](https://github.com/near/near-sdk-rs/pull/1244))

## [5.3.0](https://github.com/near/near-sdk-rs/compare/near-contract-standards-v5.2.1...near-contract-standards-v5.3.0) - 2024-08-13

### Fixed
- Fix storage management error message with proper amount ([#1222](https://github.com/near/near-sdk-rs/pull/1222))

## [5.2.0](https://github.com/near/near-sdk-rs/compare/near-contract-standards-v5.1.0...near-contract-standards-v5.2.0) - 2024-07-04

### Added
- Exported `ext_storage_management` Promise shortcuts, so Storage Management interfaces can be used in contracts to call external contracts using the high-level cross-contract call interfaces ([#1208](https://github.com/near/near-sdk-rs/pull/1208))
- Exported `ext_nft_*` Promise shortcuts, so NFT interfaces can be re-used in contracts to call external NFT contracts using the high-level cross-contract call interfaces ([#1206](https://github.com/near/near-sdk-rs/pull/1206))

## [5.1.0](https://github.com/near/near-sdk-rs/compare/near-contract-standards-v5.0.0...near-contract-standards-v5.1.0) - 2024-03-28

### Added
- Finalize `#[near]` attribute-macro implementation with the support for custom parameters passing to serializer attributes `#[near(serializers = [borsh(...)])]` ([#1158](https://github.com/near/near-sdk-rs/pull/1158))
- Introduce `#[near]` macro to further streamline contracts development reducing the boilerplate! ([#1142](https://github.com/near/near-sdk-rs/pull/1142))

## [5.0.0-alpha.3](https://github.com/near/near-sdk-rs/compare/near-contract-standards-v5.0.0-alpha.2...near-contract-standards-v5.0.0-alpha.3) - 2024-02-19

### Fixed
- Fixed a typo in the storage_deposit refund computation (introduced in 5.0.0-alpha.1 release) ([#1146](https://github.com/near/near-sdk-rs/pull/1146))

## [5.0.0-alpha.2](https://github.com/near/near-sdk-rs/compare/near-contract-standards-v5.0.0-alpha.1...near-contract-standards-v5.0.0-alpha.2) - 2024-01-16

### Other
- fix new 1.75 warnings ([#1128](https://github.com/near/near-sdk-rs/pull/1128))
- Re-exported packages cleanup ([#1114](https://github.com/near/near-sdk-rs/pull/1114))

## [5.0.0-alpha.1](https://github.com/near/near-sdk-rs/compare/4.1.1...near-contract-standards-v5.0.0-alpha.1) - 2023-11-18

### Added
- adding `nep-0330` contract source metadata info ([#1106](https://github.com/near/near-sdk-rs/pull/1106))

### Fixed
- remove receiver approval ([#1020](https://github.com/near/near-sdk-rs/pull/1020))
- rename param `approvals` to `approved_account_ids` ([#1019](https://github.com/near/near-sdk-rs/pull/1019))
- Properly report an error when Approval Extension is not enabled vs when account is not approved ([#1021](https://github.com/near/near-sdk-rs/pull/1021))

### Other
- [**breaking**] Use type-safe NearToken instead of u128/U128 ([#1104](https://github.com/near/near-sdk-rs/pull/1104))
- migrate to a external near-account-id crate for reusable AccountId type ([#1108](https://github.com/near/near-sdk-rs/pull/1108))
- Update borsh to 1.0.0 ([#1075](https://github.com/near/near-sdk-rs/pull/1075))
- Move from Gas to NearGas from near-gas crate ([#1082](https://github.com/near/near-sdk-rs/pull/1082))
- Deprecate Fungible Token declarative macros. ([#1054](https://github.com/near/near-sdk-rs/pull/1054))
- Add release-plz to automate releases ([#1069](https://github.com/near/near-sdk-rs/pull/1069))
- *(contract-standards)* deprecate declarative macros in NFT helpers, promote explicit trait implementations instead ([#1042](https://github.com/near/near-sdk-rs/pull/1042))
- Added a default method for TokenMetadata ([#978](https://github.com/near/near-sdk-rs/pull/978))
- Removed the not ready enum type ([#977](https://github.com/near/near-sdk-rs/pull/977))
- Fix empty owner tokens `start_index` error ([#962](https://github.com/near/near-sdk-rs/pull/962))
