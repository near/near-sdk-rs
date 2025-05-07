# Lockable Fungible token

Lockable Fungible token but designed for composability in the async runtime like NEAR.

It's an extension of a Fungible Token Standard (NEP#21) with locks.
Locks allow composability of the contracts, but require careful GAS management, because the token contract itself
doesn't guarantee the automatic unlocking call. That's why it shouldn't be used in production
until Safes are implemented from (NEP#26).

## Install `cargo-near` build tool

See [`cargo-near` installation](https://github.com/near/cargo-near#installation)

## Build with:

```bash
cargo near build non-reproducible-wasm
```

## Create testnet dev-account:

```bash
cargo near create-dev-account
```

## Deploy to dev-account:

```bash
cargo near deploy build-non-reproducible-wasm
```

# Demo reproducible build (in docker container):

```bash
cargo near build reproducible-wasm --no-locked
```

For a non-demo reproducible build/deploy a specific Cargo.lock has to be committed to git,
which is not done for demo examples in order to optimize maintenance burden.


## Testing
To test run:
```bash
cargo test
```
