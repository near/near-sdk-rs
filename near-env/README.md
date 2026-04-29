# near-env

Low-level abstraction over [`near-sys`](../near-sys/) host functions, providing the same
API on and off chain:

- wasm32 (contract builds): routed through NEAR VM host calls
- non-wasm32 (off-chain tools, tests): fallback to Rust implementations (`sha2`, `sha3`, `ripemd`)

Please be mindful that some of the functions like `random_seed_array()` or `block_height()` that are available in `near-sdk` might not be available in `near-env`, as they require direct interaction with blockchain and it would be impossible to reliably mimic this behaviour for off-chain services.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE_APACHE](../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../LICENSE-MIT))
