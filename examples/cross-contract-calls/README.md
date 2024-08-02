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

### Usage

```markdown
## Usage

1. Deploy the contracts:

   - Deploy the high-level contract.
   - Deploy the low-level contract.

2. Call the high-level contract to initiate a cross-contract call to the low-level contract.

## Dependencies

- [near-sdk-rs](https://github.com/near/near-sdk-rs): NEAR Protocol's Rust SDK.
- [Cargo](https://doc.rust-lang.org/cargo/): Rust's package manager.

## Changelog

### `1.0.0`

- Initial implementation of cross-contract call functionality.
```
