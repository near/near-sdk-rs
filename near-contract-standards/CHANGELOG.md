# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
