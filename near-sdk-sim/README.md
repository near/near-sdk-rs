# NEAR Simulator & cross-contract testing library

When writing NEAR contracts, with Rust or other Wasm-compiled languages like [AssemblyScript](https://github.com/near/near-sdk-as), the default testing approach for your language of choice (such as `mod test` in your Rust project's `src/lib.rs` file) is great for testing the behavior of a single contract in isolation.

But the true power of blockchains & smart contracts comes from cross-contract calls. How do you make sure your cross-contract code works as you expect?

As a first step, you can use this library! With it, you can:

* Test cross-contract calls
* Accurately check [gas](https://docs.near.org/docs/concepts/gas) & [storage](https://docs.near.org/docs/concepts/storage-staking) usage for your contract
* Inspect intermediate state of all calls in a complicated chain of transactions

To view this documentation locally, clone this repo and from this folder run `cargo doc --open`.

# Getting started

This section will guide you through our suggested approach to adding simulation tests to your project. Want an example? Check out the [Fungible Token Example](https://github.com/near/near-sdk-rs/tree/master/examples/fungible-token).

## Dependency versions

Currently this crate depends on a the GitHub repo of [nearcore](https://github.com/near/nearcore), so this crate must be a git dependency too. Furthermore, this crate's dependencies conflict with building the Wasm smart contract, so you must add it under the following:

```toml
[dev-dependencies]
near-sdk-sim = { git = "https://github.com/near/near-sdk-rs.git", tag="3.0.0" }

```

And update `near-sdk` too:

```toml
[dependencies]
near-sdk = { git = "https://github.com/near/near-sdk-rs.git", tag="3.0.0" }

```

Note that you need to add the `tag` (or `commit`) of the version.


## Workspace setup

If you want to check gas & storage usage of one Rust contract, you can add the above dependencies to `Cargo.toml` in the root of your project. If you want to test cross-contract calls, we recommend setting up a cargo [workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html). Here's how it works:

Let's say you have an existing contract project in a folder called `contract`.

Go ahead and make a subfolder within it called `contract`, and move the original contents of `contract` into this subfolder. Now you'll have `contract/contract`. You can rename the root folder to something like `contracts` or `contract-wrap`, if you want. Some bash commands to do this:

```bash
mkdir contract-wrap
mv contract contract-wrap
```

Now in the root of the project (`contract-wrap`), create a new `Cargo.toml`. You'll have to add the normal `[package]` section, but unlike most projects you won't have any `dependencies`, only `dev-dependencies` and a `workspace`:


```toml
[dev-dependencies]
near-sdk = { git = "https://github.com/near/near-sdk-rs.git", tag="3.0.0" }
near-sdk-sim = { git = "https://github.com/near/near-sdk-rs.git", tag="3.0.0" }
contract = { path = "./contract" }

[workspace]
members = [
  "contract"
]
```

Now when you want to create test contracts, you can add a new subfolder to your project and add a line for it to both `[dev-dependencies]` and `[workspace]` in this root `Cargo.toml`.

Other cleanup:

* You can move any `[profile.release]` settings from your nested project up to the root of the workspace, since workspace members inherit these settings from the workspace root.
* You can remove the nested project's `target`, since all workspace members will be built to the root project's `target` directory
* If you were building with `cargo build`, you can now build all workspace members at once with `cargo build --all`


## Test files

In the root of your project (`contract-wrap` in the example above), create a `tests` directory with a Rust file inside. Anything in here will automatically be run by `cargo test`.

Inside this folder, set up a new test crate for yourself by creating a `tests/sim` directory with a `tests/sim/main.rs` file. This file will glue together the other files (aka modules) in this folder. We'll add things to it soon. For now you can leave it empty.

Now create a `tests/sim/utils.rs` file. This file will export common functions for all your tests. In it you need to include the bytes of the contract(s) you want to test:

```rust
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    // update `contract.wasm` for your contract's name
    CONTRACT_WASM_BYTES => "target/wasm32-unknown-unknown/release/contract.wasm",

    // if you run `cargo build` without `--release` flag:
    CONTRACT_WASM_BYTES => "target/wasm32-unknown-unknown/debug/contract.wasm",
}
```

Note that this means **you must `build` before you `test`!** Since `cargo test` does not re-generate the `wasm` files that your simulation tests rely on, you will need to `cargo build --all --target wasm32-unknown-unknown` before running `cargo test`. If you made contract changes and _you swear it should pass now_, try rebuilding!

Now you can make a function to initialize your simulator:

```rust
use near_sdk_sim::{init_simulator, to_yocto, STORAGE_AMOUNT};

const CONTRACT_ID: &str = "contract";

pub fn init() -> (UserAccount, UserAccount, UserAccount) {
    // Use `None` for default genesis configuration; more info below
    let root = init_simulator(None);

    let contract = root.deploy(
        &CONTRACT_WASM_BYTES,
        CONTRACT_ID.to_string(),
        STORAGE_AMOUNT // attached deposit
    );

    let alice = root.create_user(
        "alice".to_string(),
        to_yocto("100") // initial balance
    );

    (root, contract, alice)
}
```

Now you can add a test file that uses this `init` function in `tests/sim/first_tests.rs`. For every file you add to this directory, you'll need to add a line to `tests/sim/main.rs`. Let's add one for both files so far:

```rust
// in tests/sim/main.rs
mod utils;
mod first_tests;
```

Now add some tests to `first_tests.rs`:

```rust
use near_sdk::serde_json::json;
use near_sdk_sim::DEFAULT_GAS;

use crate::utils::init;

#[test]
fn simulate_some_view_function() {
    let (root, contract, _alice) = init();

    let actual: String = root.view(
        contract.account_id(),
        "view_something",
        &json!({
            "some_param": "some_value".to_string(),
        }).to_string().into_bytes(),
    ).unwrap_json();

    assert_eq!("expected".to_string(), actual);
}

#[test]
fn simulate_some_change_method() {
    let (root, contract, _alice) = init();

    let result = root.call(
        contract.account_id(),
        "change_something",
        json!({
            "some_param": "some_value".to_string(),
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        1, // deposit
    );

    assert!(result.is_ok());
}
```


## Optional macros

The above approach is a good start, and will work even if your Wasm files are compiled from a language other than Rust.

But if your original files are Rust and you want better ergonomics while testing, `near-sdk-sim` provides a nice bonus feature.

`near-sdk-sim` modifies the `near_bindgen` macro from `near-sdk` to create an additional struct+implementation from your contract, with `Contract` added to the end of the name, like `xxxxxContract`. So if you have a contract with `[package].name` set to `token` with this in its `src/lib.rs`:

```rust
#[near_bindgen]
struct Token {
    ...
}

#[near_bindgen]
impl Token {
    ...
}
```

Then in your simulation test you can import `TokenContract`:

```rust
use token::TokenContract;

// or rename it maybe
use token::TokenContract as OtherNamedContract;
```

Now you can simplify the `init` & test code from the previous section:

```rust
// in utils.rs
use near_sdk_sim::{deploy, init_simulator, to_yocto, STORAGE_AMOUNT};
use token::TokenContract;

const CONTRACT_ID: &str = "contract";

pub fn init() -> (UserAccount, ContractAccount<TokenContract>, UserAccount) {
    let root = init_simulator(None);

    let contract = deploy!(
        contract: TokenContract,
        contract_id: CONTRACT_ID,
        bytes: &CONTRACT_WASM_BYTES,
        signer_account: root,
    );

    let alice = root.create_user(
        "alice".to_string(),
        to_yocto("100") // initial balance
    );

    (root, contract, alice)
}

// in first_tests.rs
use near_sdk_sim::{call, view};
use crate::utils::init;

#[test]
fn simulate_some_view_function() {
    let (root, contract, _alice) = init();

    let actual: String = view!(
        contract.view_something("some_value".to_string()),
    ).unwrap_json();

    assert_eq!("expected", actual);
}

#[test]
fn simulate_some_change_method() {
    let (root, contract, _alice) = init();

    // uses default gas amount
    let result = call!(
        root,
        contract.change_something("some_value".to_string()),
        deposit = 1,
    );

    assert!(result.is_ok());
}
```

# Common patterns

## Profile gas costs

TODO


## Profile storage costs

TODO


## Inspect intermediate state of all calls in a complicated chain of transactions 

TODO


## Check expected transaction failures

TODO


# Tweaking the genesis config

For many simulation tests, using `init_simulator(None)` is good enough. This uses the [default genesis configuration settings](https://github.com/near/near-sdk-rs/blob/0a9a56f1590e1f19efc974160c88f32efcb91ef4/near-sdk-sim/src/runtime.rs#L59-L72).

## Why would you want to change these?

TODO

## How do you change them?

TODO
