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

### Usage

```markdown
## Usage

1. Deploy the factory contract.
2. Use the factory contract to create instances of the target contract.

## Dependencies

- [near-sdk-rs](https://github.com/near/near-sdk-rs): NEAR Protocol's Rust SDK.
- [Cargo](https://doc.rust-lang.org/cargo/): Rust's package manager.

## Changelog

### `1.0.0`

- Initial implementation of factory contract functionality.
```
