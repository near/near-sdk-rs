Non-fungible Token (NFT)
===================

Example implementation of a [Non-Fungible Token] contract which uses [near-contract-standards].

  [Non-Fungible Token]: https://nomicon.io/Standards/NonFungibleToken
  [near-contract-standards]: https://github.com/near/near-sdk-rs/tree/master/near-contract-standards

NOTES:
 - The maximum balance value is limited by U128 (2**128 - 1).
 - JSON calls should pass [U128](https://docs.rs/near-sdk/latest/near_sdk/json_types/struct.U128.html) or [U64](https://docs.rs/near-sdk/latest/near_sdk/json_types/struct.U64.html) as a base-10 string. E.g. "100".
 - The core NFT standard does not include escrow/approval functionality, as `nft_transfer_call` provides a superior approach. Please see the approval management standard if this is the desired approach.

## Install `cargo-near` build tool

See [`cargo-near` installation](https://github.com/near/cargo-near#installation)

## Build with:

```bash
pushd nft
cargo near build non-reproducible-wasm
popd
pushd test-approval-receiver 
cargo near build non-reproducible-wasm
popd
pushd test-token-receiver 
cargo near build non-reproducible-wasm
popd
```

## Create testnet dev-account:

```bash
cargo near create-dev-account # 3 times
```

## Deploy to dev-account:

```bash
pushd nft
cargo near deploy build-non-reproducible-wasm
popd
pushd test-approval-receiver 
cargo near deploy build-non-reproducible-wasm
popd
pushd test-token-receiver 
cargo near deploy build-non-reproducible-wasm
popd
```

# Demo reproducible build (in docker container):

```bash
pushd nft
cargo near build reproducible-wasm --no-locked
popd
pushd test-approval-receiver 
cargo near build reproducible-wasm --no-locked
popd
pushd test-token-receiver 
cargo near build reproducible-wasm --no-locked
popd
```

For a non-demo reproducible build/deploy a specific Cargo.lock has to be committed to git,
which is not done for demo examples in order to optimize maintenance burden.

## Testing
To test run:
```bash
cargo test
```
