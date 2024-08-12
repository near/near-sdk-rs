# Cross Contract Call

Example implementation of a cross-contract call using [near-sdk-rs].

[near-sdk-rs]: https://github.com/near/near-sdk-rs

NOTES:

- This example demonstrates how to call another contract from within a contract.
- Ensure that the called contract is deployed and accessible.

## Project Structure

- `.cargo`: Configuration files for Cargo, the Rust package manager.
- `high-level/src`: Source code for the high-level cross-contract call example.
- `low-level/src`: Source code for the low-level cross-contract call example.
- `res`: Compiled contract binaries.
- `tests`: Integration tests for the cross-contract call examples.

## Building

To build run:

```bash
./build.sh
```

## Testing

To test run:

```bash
cargo test --package cross-contract-call -- --nocapture
```

## Usage

1. Prerequisites:

- Install cargo-near:

```bash
cargo install cargo-near
```

- Create a new testnet account:

```bash
cargo near create-dev-account
```

2. Deploy the contracts:

- Build and deploy the high-level contract:

```bash
cargo near deploy --wasmFile path/to/high-level-contract.wasm --accountId your-account.testnet
```

- Build and deploy the low-level contract:

```bash
cargo near deploy --wasmFile path/to/low-level-contract.wasm --accountId your-account.testnet
```

3. Initiate a cross-contract call:

- Call the high-level contract to initiate a cross-contract call to the low-level contract:

```bash
near call your-high-level-contract-account.testnet call-function '{"args": "value"}' --accountId your-account.testnet
```

NOTE: Replace `path/to/high-level-contract.wasm` and `path/to/low-level-contract.wasm` with the actual paths to your compiled WebAssembly files, and `your-account.testnet` with your actual NEAR testnet account ID.

## Dependencies

- [near-sdk-rs](https://github.com/near/near-sdk-rs): NEAR Protocol's Rust SDK.
- [Cargo](https://doc.rust-lang.org/cargo/): Rust's package manager.

## Changelog

### `1.0.0`

- Initial implementation of cross-contract call functionality.
