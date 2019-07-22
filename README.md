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
      <a href="https://github.com/nearprotocol/near-bindgen#run-the-contract">Run the Contract</a>
      <span> | </span>
      <a href="https://github.com/nearprotocol/near-bindgen#limitations-and-future-work">Limitations and Future Work</a>
    </h3>
</div>

## Example

Wrap a struct in `#[near_bindgen]` and it generates a smart contract compatible with the NEAR blockchain:
```rust
#[near_bindgen]
#[derive(Default, Serialize, Deserialize)]
pub struct StatusMessage {
    records: HashMap<Vec<u8>, String>,
}

#[near_bindgen]
impl StatusMessage {
    pub fn set_status(&mut self, message: String) {
        let account_id = ENV.originator_id();
        self.records.insert(account_id, message);
    }

    pub fn get_status(&self, account_id: Vec<u8>) -> Option<String> {
        self.records.get(&account_id).cloned()
    }
}
```

## Features

* **Unit-testable.** Writing unit tests is easy with `near-bindgen`:

    ```rust
    #[test]
    fn set_get_message() {
        ENV.set(Box::new(MockedEnvironment::new()));
        let account_id = b"alice";
        ENV.as_mock().set_originator_id(account_id.to_vec());
        let mut contract = StatusMessage::default();
        contract.set_status("Hello".to_owned());
        assert_eq!(Some("Hello".to_owned()), contract.get_status(account_id.to_vec()));
    }
    ```

    To run unit tests include `env_test` feature:
    ```bash
    cargo test --package status-message --features env_test
    ```

* **Asynchronous cross-contract calls.** Asynchronous cross-contract calls allow parallel execution
    of multiple contracts in parallel with subsequent aggregation on another contract.
    `ENV` exposes the following methods:
    * `promise_create` -- schedules an execution of a function on some contract;
    * `promise_then` -- attaches the callback back to the current contract once the function is executed;
    * `promise_and` -- combinator, allows waiting on several promises simultaneously, before executing the callback;
    * `return_promise` -- treats the result of execution of the promise as the result of the current function.


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

To test Rust contracts you would need a locally running NEAR testnet, which we launch
using [Docker](https://www.docker.com/products/docker-desktop).

To communicate with the NEAR network we recommend using the [NEAR shell](https://github.com/nearprotocol/near-shell)
which you can install with:
```bash
npm install -g near-shell
```

## Writing Rust Contract
You can follow the [test-contract](test-contract) crate that shows a simple Rust contract.

The general workflow is the following:
1. Create a crate and configure the `Cargo.toml` similarly to how it is configured in [test-contract/Cargo.toml](test-contract/Cargo.toml);
2. Crate needs to have one `pub` struct that will represent the smart contract itself:
    * The struct needs to implement `Default` trait which
    NEAR will use to create the initial state of the contract upon its first usage;
    * The struct also needs to implement `Serialize` and `Deserialize` traits which NEAR will use to save/load contract's internal state;
    
   Here is an example of a smart contract struct:
   ```rust
   #[near_bindgen]
   #[derive(Default, Serialize, Deserialize)]
   pub struct MyContract {
       data: HashMap<u64, u64>
   }
   ```

3. Define methods that NEAR will expose as smart contract methods:
    * Are you free to define any methods for the struct but only non-static public methods will be exposed as smart contract methods;
    * Methods need to use either `&self` or `&mut self`;
    * Decorate the `impl` section with `#[near_bindgen]` macro. That is where all the M.A.G.I.C. (Macros-Auto-Generated Injected Code) is happening 
    
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
We can build the contract using the `wasm-pack` like this:
```bash
wasm-pack build --no-typescript --release
```
This will build the contract code in the `pkg` subfolder.

The error messages are currently WIP, so please reach directly to the maintainers until this is fixed.

## Run the Contract
If you skipped the previous steps you can use the already built contract from [test-contract/res/mission_control.wasm](test-contract/res/mission_control.wasm).

Let's start the local NEAR testnet and run the smart contract on it.

* Start the local testnet:
    ```bash
    ./start_local_network.sh
    ```
* Create an account into which we will deploy the contract using NEAR shell:
    ```bash
    near create_account --useDevAccount "missioncontrol"
    ```
* Deploy the smart contract code:
    ```bash
    near deploy --accountId missioncontrol --wasmFile test-contract/res/mission_control.wasm
    ```
* Let's create some other account that will be calling our contract:
    ```bash
    near create_account --useDevAccount "purplebot"
    ```
* Using `purplebot` account call the method of the smart contract:
    ```bash
    near call missioncontrol add_agent "{}" --accountId purplebot
    ```
* Using `missioncontrol` account call its own method:
    ```bash
    near call missioncontrol simulate "{\"account_id\":\"purplebot\"}" --accountId missioncontrol
    ```
    Observe that the returned result is `true` which corresponds to the bot being "alive".
* Using `purplebot` account call the view method of the smart contract:
    ```bash
    near view missioncontrol assets_quantity "{\"account_id\":\"purplebot\",\"asset\":\"MissionTime\"}" --accountId missioncontrol
    ```
    Observe that the returned result is `2` (which is the correct expected value).
    
Note, smart contract methods that use `&mut self` modify the state of the smart contract and therefore the only way for
them to be called is using `near call` which results in a transaction being created and propagated into a block.
On the other hand, smart contract methods `&self` do not modify the state of the smart contract and can be also called with
`near view` which does not create transactions.

Note, currently NEAR shell creates a `neardev` folder with public and secret keys. You need to cleanup this folder
after you restart the local NEAR testnet.

## Limitations and Future Work
The current implementation of `wasm_bindgen` has the following limitations:
* The smart contract struct should be serializable with [bincode](https://crates.io/crates/bincode) which is true for most of the structs;
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

