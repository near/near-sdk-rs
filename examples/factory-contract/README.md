## Install `cargo-near` build tool

See [`cargo-near` installation](https://github.com/near/cargo-near#installation)

## Build with:

```bash
pushd high-level
cargo near build non-reproducible-wasm
popd
pushd low-level 
cargo near build non-reproducible-wasm
popd
```

## Create testnet dev-account:

```bash
cargo near create-dev-account # twice
```

## Deploy to dev-account:

```bash
pushd high-level
cargo near deploy build-non-reproducible-wasm
popd
pushd low-level 
cargo near deploy build-non-reproducible-wasm
popd
```

# Demo reproducible build (in docker container):

```bash
pushd high-level
cargo near build reproducible-wasm --no-locked
popd
pushd low-level 
cargo near build reproducible-wasm --no-locked
popd
```

For a non-demo reproducible build/deploy a specific Cargo.lock has to be committed to git,
which is not done for demo examples in order to optimize maintenance burden.
