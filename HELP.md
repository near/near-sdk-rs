# Contract security checklist

## Init once

Make sure you check that state doesn't exists in `init` using

```rust
assert!(!env::state_exists(), "The contract is already initialized");
```


## Prevent Default

By default `near_sdk` allows contract to be initialized with default state. Usually, if you have a constructor, you want to
prevent default state initialization.

```rust
impl Default for Contract {
    fn default() -> Self {
        env::panic(b"The contract should be initialized before usage")
    }
}
```


## Public vs private methods

`pub fn` makes a method public and exposes it in a contract. It means anyone can call it.
`fn` makes the method private and it's not exposed from the contract. No one can call it directly. It can only be called
within a contract directly (not through a promise).


## Private callbacks

If you're using callbacks, makes sure you check the caller.

```rust
assert_eq!(env::current_account_id(), env::predecessor_account_id(), "Can only be called within a contract");
```

Callbacks have to be public methods.


## JSON types

NEAR currently expects contracts to support JSON serialization. JSON can't handle large integers (above 2**53 bits).
That's why you should use helper classes from the `json_types` in `near_sdk` for `u64` and `u128`.
We provide types `U64` and `U128`, that wraps the integer into a struct and implements JSON serialization and
deserialization as a base-10 strings.

E.g.

```rust
pub fn add(&self, a: U128, b: U128) -> U128 {
    (a.0 + b.0).into()
}
```


## View vs Change method

By default, `near_sdk` assumes that the method is a `view` if it has `&self` and method is `change` if it has `&mut self`.
View methods usually don't have ability to change state and have limited context.
Change methods will save the modified state back to the contract.


## `STATE` storage key

The contract will save the main structure state under the storage key `STATE`.
Make sure you don't modify it e.g. through collection prefixes or raw storage access.


## Enable overflow checks

It's usually helpful to panic on integer overflow. To enable it, add the following into your `Cargo.toml` file:

```toml
[profile.release]
overflow-checks = true
```


## Use `assert!` early in the method.

Try to validate the input, context, state and access first before making any actions. This will safe gas for the caller to fail earlier.


## Use `env::log`

Use logging for debugging and notifying user.
When you need a formatted message, you can use the following:

```rust
env::log(format!("Transferred {} tokens from {} to {}", amount, sender_id, receiver_id).as_bytes());
```

## Return `Promise`

If your method makes a cross-contract call, you may want to return the `Promise` that the contract creates.
It allows the caller to wait for the result of the promise instead of the returning the result immediately.
Because if the promise fails for some reason, the caller will not know about this or the caller may display the result earlier.

E.g.

```rust
pub fn withdraw(&mut self, amount: U128) -> Promise {
    Promise::new(self.owner_id.clone()).transfer(amount.0)
}
```
