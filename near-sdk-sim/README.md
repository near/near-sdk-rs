# Near Simulator

This crate contains tools for simulating transactions on NEAR protocol.
It also allows testing cross contract calls and inspecting the intermediate state of the transactions
involved.

# Documentation
To view locally, clone this repo and from this folder run `cargo doc --open`.

## Setup

Currently this crate depends on a the github repo of [nearcore](https://github.com/near/nearcore), so this crate must be a git dependency too.
Furthermore, this crate's dependencies conflict with building the Wasm smart contract, so you must add it under the following:

```toml
[dev-dependencies]
near-sdk-sim = { git = "https://github.com/near/near-sdk-rs.git", tag="2.0.0" }

```

Note that you need to add the tag of the version.

### Usage

Create a `tests` directory and then a rust file. The `near_bindgen` macro has been updated to create a new struct and implementation, with `Contract` added to the end of the name:

```rust
struct TokenContract {
    account_id: String
}

impl TokenContract {
  pub fn transfer(&self, amount: U128) -> near_sdk::PendingContractTransaction {
     //...
  }
}
```

Thus you need to import this new type into your test file like so:

```rust
extern crate token;
use token::TokenContract;
```

Next you need to import the simulator's init method and some other imports.

```rust
use near_sdk_sim::{init_simulator, deploy, call, to_yocto};
```

This function takes an optional parameter of a genesis configuration. 
Next you must include the bytes of the contract you want to test.

```rust
near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/token.wasm").as_ref();
}
```
Next we initialize the simulator and then use the deploy macro to deploy and 
initialize a contract.  Next we use the call macro
```rust
#[test]
fn simple_test() {
      let master_account = init_simulator(None);
      let initial_balance = to_yocto("100000");
          // uses default values for deposit and gas
      let contract_user = deploy!(
          // Contract Proxy
          contract: FungibleTokenContract,
          // Contract account id
          contract_id: "contract",
          // Bytes of contract
          bytes: &TOKEN_WASM_BYTES,
          // User deploying the contract,
          signer_account: master_account,
          // init method
          init_method: new(master_account.account_id(), initial_balance.into())
      );
    let alice = master_account.create_user("alice".to_string(), to_yocto("100"));

    let res = call!(
        master_account,
        contract.transfer(alice.account_id.clone(), transfer_amount.into()),
        deposit = STORAGE_AMOUNT
    );

    assert!(res.is_ok());
    
    let value = view!(contract.get_balance(master_account.account_id()));
    let value: U128 = value.from_json_value().unwrap();
    assert_eq!(initial_balance - transfer_amount, value.0);
}
```

