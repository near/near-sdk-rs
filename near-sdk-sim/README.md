# Near Simulator

This crate contains tools for simulating transactions on NEAR protocol.
It also allows testing cross contract calls and inspecting the intermediate state of the transactions
involved.

## Setup

Currently this crate depends on a the github repo of [nearcore](https://github.com/near/nearcore), so this crate must be a git dependency too.
Furthermore, this crate's dependencies conflict with building the Wasm smart contract, so you must add it under the following:

```toml
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
near-sdk-sim = { git = "https://github.com/near/near-sdk-rs.git", tag="2.0.0" }

```

Note that you need to add the tag of the release you want instead of using cargo's version.

### Usage

Create a `tests` directory and then a rust file. The `near_bindgen` macro now creates a new struct and implementation, with `Contract` added to the end of the name:

```rust
struct TokenContract {
    account_id: String;
}

impl TokenContract {
  pub fn transfer(&self, amount: U128) -> near_sdk::PendingContractTransaction{
     ...
  }
}
```

Thus you need to import this new type into your test file like so:

```rust
extern crate token;
use token::TokenContract;
```

Next you need to import the simulatior's init method:

```rust
use near_sdk_sim::{init_test_runtime};
```

This function takes an optional parameter of a genesis configuration. Next you must include the bytes of the contract you want to test.

```rust
near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/token.wasm").as_ref();
}
```

```rust
#[test]
fn simple_test() {
      let runtime = init_test_runtime(None); // Passing None uses the default config
      let root = runtime.get_root();
      let initial_balance = near_sdk_sim::to_yocto("100");
      let contract = TokenContract { account_id: "contract".to_string() };
      // Or with constructor
      let contract = TokenContract::_new("contract".to_string());
      let contract_user = root.deploy_and_init(
          &TOKEN_WASM_BYTES,
          contract.new(root.account_id.clone(), initial_balance.into()),
      );
}
```

