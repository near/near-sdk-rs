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
    <a href="https://travis-ci.com/nearprotocol/near-bindgen"><img src="https://travis-ci.com/nearprotocol/near-bindgen.svg?branch=master" alt="Travis Build" /></a>
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
      <a href="https://github.com/nearprotocol/near-bindgen#running-rust-contract">Running Rust Contract</a>
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
    pub fn set_status(&mut self, message: String) {
        let account_id = env::signer_account_id();
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
        let context = get_context(vec![]);
        let config = Config::default();
        testing_env!(context, config);
        let mut contract = StatusMessage::default();
        contract.set_status("hello".to_string());
        assert_eq!("hello".to_string(), contract.get_status("bob.near".to_string()).unwrap());
    }
    ```

    Run unit test the usual way:
    ```bash
    cargo test --package status-message
    ```

* **Asynchronous cross-contract calls.** Asynchronous cross-contract calls allow parallel execution
    of multiple contracts in parallel with subsequent aggregation on another contract.
    `env` exposes the following methods:
    * `promise_create` -- schedules an execution of a function on some contract;
    * `promise_then` -- attaches the callback back to the current contract once the function is executed;
    * `promise_and` -- combinator, allows waiting on several promises simultaneously, before executing the callback;
    * `promise_return` -- treats the result of execution of the promise as the result of the current function.
    
    Follow [examples/cross-contract-high-level](https://github.com/nearprotocol/near-bindgen/tree/master/examples/cross-contract-high-level)
    to see various usages of cross contract calls, including **system-level actions** done from inside the contract like balance transfer (examples of other system-level actions are: account creation, access key creation/deletion, contract deployment, etc).

* **Initialization methods.** We can define an initialization method that can be used to initialize the state of the
contract.

    ```rust
    #[near_bindgen(init => new)]
    impl StatusMessage {
      pub fn new(user: String, status: String) -> Self {
          let mut res = Self::default();
          res.records.insert(user, status);
          res
      }
    }
    ```


## Pre-requisites
To develop Rust contracts you would need to:
* Install [Rustup](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
* Add wasm target to your toolchain:
```bash
rustup target add wasm32-unknown-unknown
```

## Writing Rust Contract
You can follow the [examples/status-message](examples/status-message) crate that shows a simple Rust contract.

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
    * If you need to use blockchain interface, e.g. to get the current account id then you can access it with `env::*`;

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
RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
```

## Running Rust Contract

If you skipped the previous steps you can use the already built contract from [examples/status-message/res/status-message.wasm](examples/status-message/res/status-message.wasm).

### Start the local testnet

Let's start the local Near testnet to run the contract on it.

* Make sure you have [Docker](https://www.docker.com/) installed;
* Clone the [nearprotocol/nearcore](https://github.com/nearprotocol/nearcore);
* Make sure you are in `master` branch, then run
    ```bash
    rm -rf testdir; ./scripts/start_unittest.py
    ```
  It might take a minute to start if you machine have not downloaded the docker image yet.

Note, the locally running node will create `testdir` directory where it will keep the node state and the configs, including
the secret key of the validator's account which we can use to create new accounts later.

### Create the project and deploy the contract
* Make sure you have the newest version of near-shell installed by running:
    ```bash
    npm install -g near-shell
    ```
* Create the near-shell project. This will allow having configuration like URL of the node in the config file instead of
passing it with each near-shell command.
    ```bash
    near new_project ./myproject; cd ./myproject
    ```
* Modify the config to point to the local node: open `./src/config.js` in `./myproject` and change `nodeUrl` under `development` to be `http://localhost:3030`.
    This is how it should look like:
    ```js
    case 'development':
    return {
       networkId: 'default',
       nodeUrl: 'http://localhost:3030',
       ...
    }
    ```

* Create account for your smart contract, e.g. we can use `status_message` as the account identifier:
    ```bash
    near create_account status_message --masterAccount=test.near --homeDir=../nearcore/testdir
    ```
    Note, `homeDir` should point to the home directory of the node which contains the secret key which we will use to sign transactions.

* Deploy the contract code to the newly created account:
    ```bash
    near deploy --accountId=status_message --homeDir=../nearcore/testdir --wasmFile=../examples/status-message/res/status_message.wasm
    ```

### Call contract functions

* Let's call the `set_status` function on the smart contract:
    ```bash
    near call status_message set_status "{\"message\": \"Hello\"}" --accountId=test.near --homeDir=../nearcore/testdir
    ```
    Notice that we use account id `test.near` to call a smart contract deployed to `status_message` account id.
    The smart contract will remember that account `test.near` left the message `"Hello"`, see the implementation in
    [examples/status-message/src/lib.rs](examples/status-message/src/lib.rs).

* Do another call to `get_status` function to check that the message was correctly recorded:
    ```bash
        near call status_message get_status "{\"account_id\": \"test.near\"}" --accountId=test.near --homeDir=../nearcore/testdir
    ```
    Observe the output:
    ```
    Result: Hello
    ```

* Do another call to `get_status` but this time inquire about the account that have not left any messages:
    ```bash
        near call status_message get_status "{\"account_id\": \"some_other_account\"}" --accountId=test.near --homeDir=../nearcore/testdir
    ```
    Observe the output:
    ```
    Result: null
    ```

### Cleaning up

* Stop the node using docker commands:
    ```bash
    docker stop nearcore watchtower
    docker rm nearcore watchtower
    ```
* Remove the node project directory:
    ```bash
    rm -rf myproject
    ```
* Remove the node data:
    ```bash
    rm -rf testdir
    ```

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
