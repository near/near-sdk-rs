Fungible Token (FT)
===================

Example implementation of a [Fungible Token] contract which uses [near-contract-standards].

  [Fungible Token]: https://nomicon.io/Standards/Tokens/FungibleToken
  [near-contract-standards]: https://github.com/near/near-sdk-rs/tree/master/near-contract-standards

NOTES:
 - The maximum balance value is limited by U128 (2**128 - 1).
 - JSON calls should pass U128 as a base-10 string. E.g. "100".
 - This does not include escrow functionality, as `ft_transfer_call` provides a superior approach. An escrow system can, of course, be added as a separate contract.

## Install `cargo-near` build tool

See [`cargo-near` installation](https://github.com/near/cargo-near#installation)

## Build with:

```bash
pushd ft
cargo near build non-reproducible-wasm
popd
pushd test-contract-defi
cargo near build non-reproducible-wasm
popd
```

## Create testnet dev-account:

```bash
cargo near create-dev-account # twice
```

## Deploy to dev-account:

```bash
pushd ft
cargo near deploy build-non-reproducible-wasm
popd
pushd test-contract-defi
cargo near deploy build-non-reproducible-wasm
popd
```

# Demo reproducible build (in docker container):

```bash
pushd ft
cargo near build reproducible-wasm --no-locked
popd
pushd test-contract-defi
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

## Changelog

### `1.0.0`

- Switched form using [NEP-21](https://github.com/near/NEPs/pull/21) to [NEP-141](https://github.com/near/NEPs/issues/141).

### `0.3.0`

#### Breaking storage change

- Switching `UnorderedMap` to `LookupMap`. It makes it cheaper and faster due to decreased storage access.

