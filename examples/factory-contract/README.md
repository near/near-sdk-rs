# Factory Contract

Example implementation of a factory contract using [near-sdk-rs].

[near-sdk-rs]: https://github.com/near/near-sdk-rs

NOTES:

- This example demonstrates how to create and manage multiple instances of a contract from a factory contract.
- Ensure that the factory contract is deployed and accessible.

## Project Structure

- `.cargo`: Configuration files for Cargo, the Rust package manager.
- `high-level/src`: Source code for the high-level factory contract example.
- `low-level/src`: Source code for the low-level factory contract example.
- `res`: Compiled contract binaries.
- `tests`: Integration tests for the factory contract examples.

## Building

To build run:

```bash
./build.sh
```

## Testing

To test run:

```bash
cargo test --package factory-contract -- --nocapture
```

## Usage

## Usage

1. Prerequisite: Install cargo-near:

```bash
cargo install cargo-near
```

2. Create a new testnet account:

```bash
cargo near create-dev-account
```

3. Deploy the factory contract:

```bash
cargo near deploy --wasmFile path/to/factory-contract.wasm --accountId your-account.testnet
```

4. Use the factory contract to create instances of the target contract:

```bash
near call your-factory-contract-account.testnet create_instance '{"args": "value"}' --accountId your-account.testnet
```

NOTE: Replace `path/to/factory-contract.wasm` with the actual path to your compiled WebAssembly file, and `your-account.testnet` with your actual NEAR testnet account ID.

## Dependencies

- [near-sdk-rs](https://github.com/near/near-sdk-rs): NEAR Protocol's Rust SDK.
- [Cargo](https://doc.rust-lang.org/cargo/): Rust's package manager.

## Changelog

### `1.0.0`

- Initial implementation of factory contract functionality.
