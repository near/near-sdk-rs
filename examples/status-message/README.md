# Status Message

Records the status messages of the accounts that call this contract.

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
