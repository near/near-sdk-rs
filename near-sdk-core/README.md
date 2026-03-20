# near-sdk-core

Core NEAR types for off-chain and on-chain usage.

This crate provides the foundational types (`PublicKey`, `AccountId`, `U128`, `Base64VecU8`, etc.) used by the [NEAR SDK](https://crates.io/near-sdk), extracted so that off-chain applications (indexers, CLIs, frontends) can use them without pulling in the full smart contract SDK.

## Usage

```toml
[dependencies]
near-sdk-core = "0.1"
```

For smart contract development, use [near-sdk](../near-sdk/README.md) directly - it re-exports everything from this crate.

## Contributing

If you are interested in contributing, please look at the [contributing guidelines](https://github.com/near/near-sdk-rs/blob/master/CONTRIBUTING.md).

## License

[MIT](../LICENSE-MIT) OR [Apache-2.0](../LICENSE-APACHE)
