# near-global-contracts

Global contract identifiers and account derivation for deterministic `0s` accounts
([NEP-616](https://github.com/near/NEPs/pull/616)) and universal `0u` accounts
([NEP-655](https://github.com/near/NEPs/pull/655)), packaged as a standalone crate so
off-chain code (indexers, CLIs, services, non-NEAR wasm runtimes) can use the same types
and produce the same account IDs that on-chain contracts do.

Contract authors using `near-sdk` get these types re-exported under
`near_sdk::state_init` and `near_sdk::universal_state_init` and do not need to depend on
this crate directly.

A `0u` id is SHA3-256 of the exact state-init bytes, and the chain accepts non-canonical
encodings, which derive a different account. `RawStateInit` holds those bytes and is what
to store, send and forward; `UniversalStateInit` builds and decodes them. Never decode and
re-encode bytes you were handed.

## Quick start

### Off-chain (indexers, CLIs, etc.)

```toml
[dependencies]
near-global-contracts = { version = "0.3", features = ["serde", "borsh"] }
```

```rust
use near_global_contracts::{StateInit, StateInitV1, GlobalContractId};

let state_init = StateInit::from(StateInitV1::code(
    GlobalContractId::AccountId("example.near".parse().unwrap()),
));
let account_id = state_init.derive_account_id();
println!("{account_id}"); // 0s<40 hex chars>

use near_global_contracts::{RawStateInit, UniversalStateInitV1};

let raw = RawStateInit::from(UniversalStateInitV1::default().with_code(
    GlobalContractId::AccountId("code.near".parse().unwrap()),
));
println!("{}", raw.derive_account_id()); // 0u<52 base32 chars>
```

### Inside an on-chain contract (via `cargo-near`)

You don't need to add this crate directly — use `near-sdk` and `cargo-near build`.
`cargo-near` sets `--cfg near`, which routes hashing through the NEAR host functions.

### Non-NEAR wasm runtimes (e.g. TEE-hosted code)

Same `Cargo.toml` as the off-chain case. Building with `cargo build --target
wasm32-unknown-unknown` (without `--cfg near`) produces a wasm binary that uses
pure-Rust hashing and does not import any NEAR host functions. You can verify with
`cargo tree --target wasm32-unknown-unknown`.

## Features

| Feature                    | Effect                                                                  |
| -------------------------- | ----------------------------------------------------------------------- |
| `serde`                    | `Serialize`/`Deserialize` impls (`RawStateInit` as base64) |
| `borsh`                    | `BorshSerialize`/`BorshDeserialize` impls (also enables the typed `derive_account_id`s) |
| `abi`                      | `schemars::JsonSchema` and `borsh::BorshSchema` for ABI tooling         |
| `arbitrary`                | `arbitrary::Arbitrary` impls for fuzzing                                |
| `near-primitives-interop`  | `From`/`Into` between these types and the `near-primitives-core` ones   |

`StateInit::derive_account_id` and `UniversalStateInit::derive_account_id` require the
`borsh` feature; `RawStateInit::derive_account_id` needs none. Hashing backend is selected
automatically: on `--cfg near` (set by `cargo-near`) it routes through NEAR host functions
(`keccak256`, `sha3_256`); otherwise it uses pure-Rust `sha3`, which is pulled in
unconditionally on the `cfg(not(near))` path.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))

at your option.
