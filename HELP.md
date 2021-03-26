# Contract best practices

## `STATE` storage key

The contract will save the main structure state under the storage key `STATE`.
Make sure you don't modify it e.g. through collection prefixes or raw storage access.

## Upgrading contract

After `3.0.1` change, `#[init]` macro initializes the contract and verifies that the old state doesn't exist.
It will panic if the old state (under key `STATE`) is present in the storage.

But if you need to re-initialize the contract STATE, you need to use `#[init(ignore_state)]` instead.
This will NOT check that the state exists and you can use it in case you need to upgrade contract and migrate state.

```rust
#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    pub fn migrate_state(new_data: String) -> Self {
        // Deserialize the state using the old contract structure.
        let old_contract: OldContract = env::state_read().expect("Old state doesn't exist");
        // Verify that the migration can only be done by the owner.
        // This is not necessarily, if the upgrade is done internally.
        assert_eq!(
            &env::predecessor_account_id(),
            &old_contract.owner_id,
            "Can only be called by the owner"
        );

        // Create the new contract using the data from the old contract.
        Self { owner_id: old_contract.owner_id, data: old_contract.data, new_data }
    }
}
```

## Use `PanicOnDefault`

By default `near_sdk` allows a contract to be initialized with the default state.
Usually, if you have an initializer, you may want to prevent it.
There is a helper derive macro `PanicOnDefault` to do this, e.g.

```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub data: String,
}
```

## Public vs private methods

For methods in the implementation under `#[near_bindgen]`:

- `pub fn` makes a method public and exports it in a contract. It means anyone can call it.
- `fn` makes the method internal and it's not exported from the contract. No one can call it directly. It can only be called
within a contract directly (not through a promise).
- `pub(crate) fn` also will make a method internal. It's helpful to use it when you have a method in a different module.

```rust
#[near_bindgen]
impl Contract {
    pub fn increment(&mut self) {
        self.internal_increment();
    }
    
    fn internal_increment(&mut self) {
        self.counter += 1;
    }
}
```

Another way of not exporting methods is by having a separate `impl Contract` section, that is not marked with `#[near_bindgen]`.

```rust
#[near_bindgen]
impl Contract {
    pub fn increment(&mut self) {
        self.internal_increment();
    }
}

impl Contract {
    /// This methods is still not exported.
    pub fn internal_increment(&mut self) {
        self.counter += 1;
    }
}
```

## Callbacks

Callbacks have to be public methods exported from the contract, they needs to be called using a function call.

If you're using callbacks, makes sure you check the predecessor to avoid someone else from calling it.

There is an macro decorator `#[private]` that checks that the current account ID is equal to the predecessor account ID.  

```rust
#[near_bindgen]
impl Contract {
    #[private]
    pub fn resolve_transfer(&mut self) {
        env::log(b"This is a callback");
    }
}
```

## JSON types

NEAR Protocol currently expects contracts to support JSON serialization. JSON can't handle large integers (above 2**53 bits).
That's why you should use helper classes from the `json_types` in `near_sdk` for `u64` and `u128`.
We provide types `U64` and `U128`, that wraps the integer into a struct and implements JSON serialization and
deserialization as a base-10 strings.

You can convert from `U64` to `u64` and back using `std::convert::Into`, e.g.

```rust
#[near_bindgen]
impl Contract {
    pub fn mult(&self, a: U64, b: U64) -> U128 {
        let a: u64 = a.into();
        let b: u64 = b.into();
        let product = u128::from(a) * u128::from(b);
        product.into()
    }
}
```

You can also access inner values and using `.0` and `U128(5)`, e.g.

```rust
#[near_bindgen]
impl Contract {
    pub fn sum(&self, a: U128, b: U128) -> U128 {
        U128(a.0 + b.0)
    }
}
```

## View vs Change method

`near_sdk` assumes that the method is a `view` if it uses `&self` or `self` and method is `change` if it has `&mut self`.

View methods don't safe the contract STATE at the end of the method execution.

Change methods will save the modified STATE at the end of the method execution. They can also modify the state in persistent collections.

Note: Change methods will also check that the function call doesn't have attached deposit, unless the method is marked with the `#[payable]` macro.

```rust
#[near_bindgen]
impl Contract {
    /// View method. Requires cloning the account id.
    pub fn get_owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// View method. More efficient, but can't be reused internally, because it consumes self. 
    pub fn get_owner_id2(self) -> AccountId {
        self.owner_id
    }

    /// Change method. Changes the state, and then saves the new state internally.
    pub fn set_owner_id(&mut self, new_owner_id: ValidAccountId) {
        self.owner_id = new_owner_id.into();
    }
}
```

## Payable methods

To mark a change method as a payable, you need add `#[payable]` macro decorator. This will allow this change method
to receive attached deposits. Otherwise, if a deposit is attached to non-payable change method, the method will panic.

```rust
#[near_bindgen]
impl Contract {
    #[payable]
    pub fn take_my_money(&mut self) {
        env::log(b"Thanks!");
    }
}
```

## Enable overflow checks

It's usually helpful to panic on integer overflow. To enable it, add the following into your `Cargo.toml` file:

```toml
[profile.release]
overflow-checks = true
```

## Use `assert!` early

Try to validate the input, context, state and access first before making any actions. This will save gas for the caller if you panic earlier.

```rust
#[near_bindgen]
impl Contract {
    pub fn set_fee(&mut self, new_fee: Fee) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Owner's method");
        new_fee.assert_valid();
        self.internal_set_fee(new_fee);
    }
}
```

## Use `log!`

Use logging for debugging and notifying user.

When you need a formatted message, you can use the following macro:

```rust
log!("Transferred {} tokens from {} to {}", amount, sender_id, receiver_id);
```

It's equivalent to the following message:

```rust
env::log(format!("Transferred {} tokens from {} to {}", amount, sender_id, receiver_id).as_bytes());
```

## Return `Promise`

If your method makes a cross-contract call, you may want to return the newly created `Promise`.
It allows the caller to wait for the result of the promise instead of the returning the result immediately.
Because if the promise fails for some reason, the caller will not know about this or the caller may display the result earlier.

E.g.

```rust
#[near_bindgen]
impl Contract {
    pub fn withdraw_100(&mut self, receiver_id: ValidAccountId) -> Promise {
        Promise::new(receiver_id.into()).transfer(100)
    }
}
```

## Use high-level cross-contract API

There is a helper macro that allows to make cross-contract calls called `#[ext_contract(...)]`. It takes a Rust Trait and
converts it to a module with static methods. Each of these static methods takes positional arguments defined by the Trait,
then the `receiver_id`, the attached deposit and the amount of gas and returns a new `Promise`.

For example, let's define a calculator contract Trait:

```rust
#[ext_contract(ext_calculator)]
trait Calculator {
    fn mult(&self, a: U64, b: U64) -> U128;

    fn sum(&self, a: U128, b: U128) -> U128;
}
```

It equivalent to the following code:

```rust
mod ext_calculator {
    pub fn mult(a: U64, b: U64, receiver_id: &AccountId, deposit: Balance, gas: Gas) -> Promise {
        Promise::new(receiver_id.clone())
            .function_call(
                b"mult",
                json!({ "a": a, "b": b }).to_string().as_bytes(),
                deposit,
                gas,
            )
    }

    pub fn sum(a: U128, b: U128, receiver_id: &AccountId, deposit: Balance, gas: Gas) -> Promise {
        // ...
    }
}
```

Let's assume the calculator is deployed on `calc.near`, we can use the following:

```rust
const CALCULATOR_ACCOUNT_ID: &str = "calc.near";
const NO_DEPOSIT: Balance = 0;
const BASE_GAS: Gas = 5_000_000_000_000;

#[near_bindgen]
impl Contract {
    pub fn sum_a_b(&mut self, a: U128, b: U128) -> Promise {
        let calculator_account_id: AccountId = CALCULATOR_ACCOUNT_ID.to_string(); 
        ext_calculator::sum(a, b, &calculator_account_id, NO_DEPOSIT, BASE_GAS)
    }
}
```

## Reuse crates from `near-sdk`

TODO

## Use `setup_alloc!`

TODO

## `panic!` vs `env::panic`

TODO

## `HashMap` vs `LookupMap`

TODO

## `LookupMap` vs `UnorderedMap`

TODO

## `LazyOption`

TODO

## `Base64VecU8` 

TODO

## Compile smaller binaries

TODO

## Use simulation testing

TODO
