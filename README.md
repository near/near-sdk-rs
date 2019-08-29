<div align="center">

  <h1><code>near-bindgen</code></h1>

  <p>
    <strong>Rust library for writing NEAR smart contracts.</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/near-bindgen"><img src="https://img.shields.io/crates/v/near-bindgen.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/near-bindgen"><img src="https://img.shields.io/crates/d/near-bindgen.svg?style=flat-square" alt="Download" /></a>
    <a href="https://spectrum.chat/near"><img src="https://withspectrum.github.io/badge/badge.svg" alt="Join the community on Spectrum" /></a>
    <a href="https://discord.gg/gBtUFKR"><img src="https://img.shields.io/discord/490367152054992913.svg" alt="Join the community on Discord" /></a>
  </p>
  
   <h3>
      <a href="https://github.com/nearprotocol/near-bindgen#features">Features</a>
      <span> | </span>
      <a href="https://github.com/nearprotocol/near-bindgen#pre-requisites">Pre-requisites</a>
      <span> | </span>
      <a href="https://github.com/nearprotocol/near-bindgen#writing-rust-contract">Writing Rust Contract</a>
      <span> | </span>
      <a href="https://github.com/nearprotocol/near-bindgen#building-rust-contract">Building Rust Contract</a>
      <span> | </span>
      <a href="https://github.com/nearprotocol/near-bindgen#limitations-and-future-work">Limitations and Future Work</a>
    </h3>
</div>

## Example

Wrap a struct in `#[near_bindgen]` and it generates a smart contract compatible with the NEAR blockchain:
```rust
#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct StatusMessage {
    records: HashMap<String, String>,
}

#[near_bindgen]
impl StatusMessage {
    pub fn set_status(&mut self, env: &mut Environment, message: String) {
        let account_id = env.signer_account_id();
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: String) -> Option<String> {
        self.records.get(&account_id).cloned()
    }
}
```

## Features

* **Unit-testable.** Writing unit tests is easy with `near-bindgen`:

    ```rust
    #[test]
    fn set_get_message() {
        // Use VMContext to setup gas, balance, storage usage, account id, etc.
        let context = VMContext { ... };
        let config = Config::default();
        testing_env!(env, context, config);
        let mut contract = StatusMessage::default();
        contract.set_status(&mut env, "hello".to_string());
        assert_eq!("hello".to_string(), contract.get_status("bob.near".to_string()).unwrap());
    }
    ```

    To run unit tests include `env_test` feature:
    ```bash
    cargo test --package status-message --features env_test
    ```

* **Asynchronous cross-contract calls.** Asynchronous cross-contract calls allow parallel execution
    of multiple contracts in parallel with subsequent aggregation on another contract.
    `Environment` exposes the following methods:
    * `promise_create` -- schedules an execution of a function on some contract;
    * `promise_then` -- attaches the callback back to the current contract once the function is executed;
    * `promise_and` -- combinator, allows waiting on several promises simultaneously, before executing the callback;
    * `promise_return` -- treats the result of execution of the promise as the result of the current function.


## Pre-requisites
To develop Rust contracts you would need have:
* [Rustup](https://rustup.rs/) installed and switched to `nightly` Rust compiler:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
```
* [WASM pack](https://rustwasm.github.io/wasm-pack/) installed:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Writing Rust Contract
You can follow the [test-contract](test-contract) crate that shows a simple Rust contract.

The general workflow is the following:
1. Create a crate and configure the `Cargo.toml` similarly to how it is configured in [examples/status-message/Cargo.toml](examples/status-message/Cargo.toml);
2. Crate needs to have one `pub` struct that will represent the smart contract itself:
    * The struct needs to implement `Default` trait which
    NEAR will use to create the initial state of the contract upon its first usage;
    * The struct also needs to implement `BorshSerialize` and `BorshDeserialize` traits which NEAR will use to save/load contract's internal state;
    
   Here is an example of a smart contract struct:
   ```rust
   #[near_bindgen]
   #[derive(Default, BorshSerialize, BorshDeserialize)]
   pub struct MyContract {
       data: HashMap<u64, u64>
   }
   ```

3. Define methods that NEAR will expose as smart contract methods:
    * You are free to define any methods for the struct but only public methods will be exposed as smart contract methods;
    * Methods need to use either `&self`, `&mut self`, or `self`;
    * Decorate the `impl` section with `#[near_bindgen]` macro. That is where all the M.A.G.I.C. (Macros-Auto-Generated Injected Code) is happening 
    * If you need to use blockchain interface, e.g. to get the current account id then add `env: &mut Environment` argument.
    
    Here is an example of smart contract methods:
    ```rust
    #[near_bindgen]
    impl MyContract {
       pub fn insert_data(&mut self, key: u64, value: u64) -> Option<u64> {
           self.data.insert(key)
       }
       pub fn get_data(&self, key: u64) -> Option<u64> {
           self.data.get(&key).cloned()
       }
    }
    ```
    
## Building Rust Contract
We can build the contract using rustc:
```bash
cargo +nightly build --target wasm32-unknown-unknown --release
```
But then we would want to compress it using `wasm-opt` and `wasm-gc` tools that we installed with [wasm-pack](https://rustwasm.github.io/wasm-pack/), see [examples/status-message/build.sh](examples/status-message/build.sh).

## Limitations and Future Work
The current implementation of `wasm_bindgen` has the following limitations:
* The smart contract struct should be serializable with [borsh](http://borsh.io) which is true for most of the structs;
* The method arguments and the return type should be json-serializable, which is true for most of the types, with some exceptions. For instance,
a `HashMap<MyEnum, SomeValue>` where `MyEnum` is a non-trivial tagged-union with field-structs in variants will not serialize into json, you would need to convert it to
`Vec<(MyEnum, SomeValue)>` first. **Require arguments and the return type to be json-serializable for compatiblity with
contracts written in other languages, like TypeScript;**
* Smart contract can use `std` but cannot use wasm-incompatible OS-level features, like threads, file system, network, etc. In the future we will support the file system too;
* Smart contracts should be deterministic and time-independent, e.g. we cannot use `Instant::now`. In the future we will expose `Instant::now`;

We also have the following temporary inefficiencies:
* Current smart contracts do not utilize the trie and do not use state storage efficiently. It is okay for small collections,
but in the future we will provide an alternative `near::collections::{HashMap, HashSet, Vec}` that will be using storage in an efficient way;
* The current smart contract size is around typically ~80-180Kb, which happens because we compile-in the `bincode` and `serde-json` libraries.
In the future, we will cherry-pick only the necessary components from these libraries.
For now you can use `wasm-opt` to slightly shrink the size:
    ```bash
    wasm-opt -Oz --output ./pkg/optimized_contract.wasm ./pkg/contract.wasm
    ```
    See Binaryen for [the installation instructions](https://github.com/WebAssembly/binaryen).

