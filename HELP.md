# Contract best practices

## Main structure and persistent collections

The main contract structure is marked with `#[near_bindgen]`. It has to be serializable and deserializable with [Borsh](https://borsh.io).

```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    pub data: String,
    pub owner_id: AccountId,
    pub value: u128,
}
```

Every time an external method is called, the entire structure has to be deserialized.
The serialized contract data is stored in [persistent storage] under the key `STATE`.

Change methods ([see below](#view-vs-change-method)) serialize the main contract structure at the end and store the new value into storage.

[Persistent collections](./near-sdk/src/collections) help store extra data in persistent storage outside of the main structure.
NEAR SDK provides the following collections:

- `Vector` - An iterable implementation of vector.
- `LookupMap` - A non-iterable implementation of a map.
- `LookupSet` - A non-iterable implementation of a set.
- `UnorderedMap` - An iterable implementation of a map.
- `UnorderedSet` - An iterable implementation of a set.
- `TreeMap` - An iterable sorted map based on AVL-tree
- `LazyOption` - An `Option` for a single value.

Every instance of a persistent collection requires a unique storage prefix.
The prefix is used to generate internal keys to store data in persistent storage.
These internal keys need to be unique to avoid collisions (including collisions with key `STATE`).

  [persistent storage]: https://nomicon.io/DataStructures/Account.html#storage

## Generating unique prefixes for persistent collections

When a contract gets complicated, there may be multiple different
collections that are not all part of the main structure, but instead part of a sub-structure or nested collections.
They all need to have unique prefixes.

We can introduce an `enum` for tracking storage prefixes and keys.
And then use borsh serialization to construct a unique prefix for every collection.
It's as efficient as manually constructing them, because with Borsh serialization, an enum only takes one byte.

```rust
use near_sdk::BorshStorageKey;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Accounts,
    SubAccount { account_hash: Vec<u8> },
    Tokens,
    Metadata,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            accounts: UnorderedMap::new(StorageKeys::Accounts),
            tokens: LookupMap::new(StorageKeys::Tokens),
            metadata: LazyOption::new(StorageKeys::Metadata),
        }
    }
    
    fn get_tokens(&self, account_id: &AccountId) -> UnorderedSet<String> {
        let tokens = self.accounts.get(account_id).unwrap_or_else(|| {
            UnorderedSet::new(
                StorageKeys::SubAccount { account_hash: env::sha256(account_id.as_bytes()) }
            )
        });
        tokens
    }
}
```

For a traditional way of handling it, see [instructions below](#the-traditional-way-of-handling-unique-prefixes-for-persistent-collections)


## Upgrading a contract

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
        // This is not necessary, if the upgrade is done internally.
        require!(
            env::predecessor_account_id() == old_contract.owner_id,
            "Can only be called by the owner"
        );

        // Create the new contract using the data from the old contract.
        Self { owner_id: old_contract.owner_id, data: old_contract.data, new_data }
    }
}
```

## Use `PanicOnDefault`

By default `near_sdk` allows a contract to be initialized with default state.
Usually, if you have an initializer, you will want to prevent this.
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

While this style of public and private methods are familiar from other programming contexts, smart contracts also have a more unique need: public methods (exported as part of the contract interface; callable via [cross-contract calls](https://docs.near.org/docs/tutorials/contracts/cross-contract-calls)) that are only callable by the contract itself (such as [`ft_resolve_transfer`](https://nomicon.io/Standards/Tokens/FungibleTokenCore.html#reference-level-explanation) for Fungible Token contracts).

We call such methods _callbacks_.

### Callbacks

Callbacks have to be public methods exported from the contract, and need to be called using a function call.

If you're using callbacks, make sure you check the predecessor to prevent someone else from calling it.

There is an macro decorator `#[private]` that checks that the current account ID is equal to the predecessor account ID.

```rust
#[near_bindgen]
impl Contract {
    #[private]
    pub fn resolve_transfer(&mut self) {
        env::log_str("This is a callback");
    }
}
```

This is equivalent to:

```rust
#[near_bindgen]
impl Contract {
    pub fn resolve_transfer(&mut self) {
        if near_sdk::env::current_account_id() != near_sdk::env::predecessor_account_id() {
            near_sdk::env::panic_str("Method resolve_transfer is private");
        }
        env::log_str("This is a callback");
    }
}
```

## Integer JSON types

NEAR Protocol currently expects contracts to support JSON serialization. JSON can't handle large integers (above `2**53` bits).
That's why, for all exported methods, you should use helper classes from the `json_types` in `near_sdk` instead of `u64` and `u128`.
We provide types `U64` and `U128`, which wrap the integer into a struct and implement JSON serialization and
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

You can also access inner values and using `.0`:

```diff
 #[near_bindgen]
 impl Contract {
     pub fn mult(&self, a: U64, b: U64) -> U128 {
-        let a: u64 = a.into();
+        let a = a.0;
-        let b: u64 = b.into();
+        let b = b.0;
         let product = u128::from(a) * u128::from(b);
         product.into()
     }
 }
```

And you can cast the lower-case `u` variants to upper-case `U` variants using `U64(...)` and `U128(...)`:

```diff
 #[near_bindgen]
 impl Contract {
     pub fn mult(&self, a: U64, b: U64) -> U128 {
         let a = a.0;
         let b = b.0;
         let product = u128::from(a) * u128::from(b);
-        product.into()
+        U128(product)
     }
 }
```

Combining it all:

```rust
#[near_bindgen]
impl Contract {
    pub fn mult(&self, a: U64, b: U64) -> U128 {
        U128(u128::from(a.0) * u128::from(b.0))
    }
}
```

## `Base64VecU8` JSON type

Contracts often need to receive or return binary data.
Encoding a `Vec<u8>` with JSON will lead to an integer array, e.g. `[110, 101, 97, 114]`
This is inefficient in both compute and space.

`Base64VecU8` is a wrapper on top of `Vec<u8>` that allows to pass it as arguments or result.

```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    // Notice, internally we store `Vec<u8>` 
    pub data: Vec<u8>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(data: Base64VecU8) -> Self {
        Self {
            data: data.into(),
        }
    }

    pub fn get_data(self) -> Base64VecU8 {
        self.data.into()
    }
}
```

## View vs Change method

`near_sdk` assumes that the method is a `view` if it uses `&self` or `self` and method is `change` if it has `&mut self`.

View methods don't save the contract STATE at the end of the method execution.
However, a view method COULD modify contract STATE or persistent collection state in-memory, knowing that all changes will be discarded after the method returns.

Change methods will automatically save the modified STATE at the end of the method execution. They can also modify the state in persistent collections.

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
    pub fn set_owner_id(&mut self, new_owner_id: AccountId) {
        self.owner_id = new_owner_id;
    }

    /// View method that "modifies" state, for code structure or computational
    /// efficiency reasons. Changes state in-memory, but does NOT save the new
    /// state. If called internally by a change method, WILL result in updated
    /// contract state.
    pub fn update_stats(&self, account_id: AccountId, score: U64) -> Account {
        let account = self.accounts.get(&account_id).expect("account not found");
        account.total += score;
        account
    }
}
```

For more information about `&self` versus `self` see the [rust book](https://doc.rust-lang.org/stable/book/ch05-03-method-syntax.html?highlight=capture%20self#defining-methods)

## Payable methods

To mark a change method as a payable, you need to add the `#[payable]` macro decorator. This will allow this change method
to receive attached deposits. Otherwise, if a deposit is attached to a non-payable change method, the method will panic.

```rust
#[near_bindgen]
impl Contract {
    #[payable]
    pub fn take_my_money(&mut self) {
        env::log_str("Thanks!");
    }

    pub fn do_not_take_my_money(&mut self) {
        env::log_str("Thanks!");
    }
}
```

This is equivalent to:

```rust
#[near_bindgen]
impl Contract {
    pub fn take_my_money(&mut self) {
        env::log_str("Thanks!");
    }

    pub fn do_not_take_my_money(&mut self) {
        if near_sdk::env::attached_deposit() != 0 {
            near_sdk::env::panic_str("Method do_not_take_my_money doesn't accept deposit");
        }
        env::log_str("Thanks!");
    }
}
```

## Enable overflow checks

It's usually helpful to panic on integer overflow. To enable it, add the following into your `Cargo.toml` file:

```toml
[profile.release]
overflow-checks = true
```

## Use `require!` early

> `near_sdk::require` is a more lightweight version of the rust `assert!` macro

Try to validate the input, context, state and access first before taking any actions. The earlier you panic, the more [gas](https://docs.near.org/docs/concepts/gas) you will save for the caller.

```rust
#[near_bindgen]
impl Contract {
    pub fn set_fee(&mut self, new_fee: Fee) {
        require!(env::predecessor_account_id() == self.owner_id, "Owner's method");
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
env::log_str(format!("Transferred {} tokens from {} to {}", amount, sender_id, receiver_id).as_ref());
```

## Return `Promise`

If your method makes a cross-contract call, you probably want to return the newly created `Promise`.
This allows the caller (such as a near-cli or near-api-js call) to wait for the result of the promise instead of returning immediately.
Additionally, if the promise fails for some reason, returning it will let the caller know about the failure, as well as enabling NEAR Explorer and other tools to mark the whole transaction chain as failing.
This can prevent false-positives when the first or first few transactions in a chain succeed but a subsequent transaction fails.

E.g.

```rust
#[near_bindgen]
impl Contract {
    pub fn withdraw_100(&mut self, receiver_id: AccountId) -> Promise {
        Promise::new(receiver_id).transfer(100)
    }
}
```

## Use high-level cross-contract API

There is a helper macro that allows you to make cross-contract calls called `#[ext_contract(...)]`. It takes a Rust Trait and
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
                "mult".to_string(),
                json!({ "a": a, "b": b }).to_string().into_bytes(),
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

`near-sdk` re-exports the following crates:

- `borsh`
- `base64`
- `bs58`
- `serde`
- `serde_json`

Most common crates include `borsh` which is needed for internal STATE serialization and
`serde` for external JSON serialization.

When marking structs with `serde::Serialize` you need to use `#[serde(crate = "near_sdk::serde")]`
to point serde to the correct base crate.

```rust
/// Import `borsh` from `near_sdk` crate 
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
/// Import `serde` from `near_sdk` crate 
use near_sdk::serde::{Serialize, Deserialize};

/// Main contract structure serialized with Borsh
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub pair: Pair,
}

/// Implements both `serde` and `borsh` serialization.
/// `serde` is typically useful when returning a struct in JSON format for a frontend.
#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Pair {
    pub a: u32,
    pub b: u32,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(pair: Pair) -> Self {
        Self {
            pair,
        }
    }

    pub fn get_pair(self) -> Pair {
        self.pair
    }
}
```

## `std::panic!` vs `env::panic`

- `std::panic!` panics the current thread. It uses `format!` internally, so it can take arguments.
  SDK sets up a panic hook, which converts the generated `PanicInfo` from `panic!` into a string and uses `env::panic` internally to report it to Runtime.
  This may provides extra debugging information such as the line number of the source code where the panic happened.

- `env::panic` directly calls the host method to panic the contract.
  It doesn't provide any other extra debugging information except for the passed message.

## In-memory `HashMap` vs persistent `UnorderedMap`

- `HashMap` keeps all data in memory. To access it, the contract needs to deserialize the whole map.
- `UnorderedMap` keeps data in persistent storage. To access an element, you only need to deserialize this element.

Use `HashMap` in case:

- Need to iterate over all elements in the collection **in one function call**.
- The number of elements is small or fixed, e.g. less than 10.

Use `UnorderedMap` in case:

- Need to access a limited subset of the collection, e.g. one or two elements per call.
- Can't fit the collection into memory.

The reason is `HashMap` deserializes (and serializes) the entire collection in one storage operation.
Accessing the entire collection is cheaper in gas than accessing all elements through `N` storage operations.

Example of `HashMap`:

```rust
/// Using Default initialization.
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct Contract {
    pub status_updates: HashMap<AccountId, String>,
}

#[near_bindgen]
impl Contract {
    pub fn set_status(&mut self, status: String) {
        self.status_updates.insert(env::predecessor_account_id(), status);
        assert!(self.status_updates.len() <= 10, "Too many messages");
    }

    pub fn clear(&mut self) {
        // Effectively iterating through all removing them.
        self.status_updates.clear();
    }

    pub fn get_all_updates(self) -> HashMap<AccountId, String> {
        self.status_updates
    }
}
```

Example of `UnorderedMap`:

```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub status_updates: UnorderedMap<AccountId, String>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        // Initializing `status_updates` with unique key prefix.
        Self {
            status_updates: UnorderedMap::new(b"s".to_vec()),
        }
    }

    pub fn set_status(&mut self, status: String) {
        self.status_updates.insert(&env::predecessor_account_id(), &status);
        // Note, don't need to check size, since `UnorderedMap` doesn't store all data in memory.
    }

    pub fn delete_status(&mut self) {
        self.status_updates.remove(&env::predecessor_account_id());
    }

    pub fn get_status(&self, account_id: AccountId) -> Option<String> {
        self.status_updates.get(&account_id)
    }
}
```

## Pagination with persistent collections

Persistent collections such as `UnorderedMap`, `UnorderedSet` and `Vector` may
contain more elements than the amount of gas available to read them all.
In order to expose them all through view calls, we can implement pagination.

`Vector` returns elements by index natively using `.get(index)`.

To access elements by index in `UnorderedSet` we can use `.as_vector()` that will return a `Vector` of elements.

For `UnorderedMap` we need to get keys and values as `Vector` collections, using `.keys_as_vector()` and `.values_as_vector()` respectively.

Example of pagination for `UnorderedMap`:

```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub status_updates: UnorderedMap<AccountId, String>,
}

#[near_bindgen]
impl Contract {
    /// Retrieves multiple elements from the `UnorderedMap`.
    /// - `from_index` is the index to start from.
    /// - `limit` is the maximum number of elements to return.
    pub fn get_updates(&self, from_index: u64, limit: u64) -> Vec<(AccountId, String)> {
        let keys = self.status_updates.keys_as_vector();
        let values = self.status_updates.values_as_vector();
        (from_index..std::cmp::min(from_index + limit, self.status_updates.len()))
            .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap()))
            .collect()
    }
}
```

## `LookupMap` vs `UnorderedMap`

### Functionality

- `UnorderedMap` supports iteration over keys and values, and also supports pagination. Internally, it has the following structures:
  - a map from a key to an index
  - a vector of keys
  - a vector of values
- `LookupMap` only has a map from a key to a value. Without a vector of keys, it doesn't have the ability to iterate over keys.

### Performance

`LookupMap` has a better performance and stores less data compared to `UnorderedMap`.

- `UnorderedMap` requires `2` storage reads to get the value and `3` storage writes to insert a new entry.
- `LookupMap` requires only one storage read to get the value and only one storage write to store it.

### Storage space

`UnorderedMap` requires more storage for an entry compared to a `LookupMap`.

- `UnorderedMap` stores the key twice (once in the first map and once in the vector of keys) and value once. It also has a higher constant for storing the length of vectors and prefixes.
- `LookupMap` stores key and value once.

## `LazyOption`

It's a type of persistent collection that only stores a single value.
The goal is to prevent a contract from deserializing the given value until it's needed.
An example can be a large blob of metadata that is only needed when it's requested in a view call,
but not needed for the majority of contract operations.

It acts like an `Option` that can either hold a value or not and also requires a unique prefix (a key in this case)
like other persistent collections.

Compared to other collections, `LazyOption` only allows you to initialize the value during initialization.

```rust
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub metadata: LazyOption<Metadata>,
}

#[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Metadata {
    data: String,
    image: Base64Vec,
    blobs: Vec<String>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata: LazyOption::new(b"m", Some(metadata)),
        }
    }

    pub fn get_metadata(&self) -> Metadata {
        // `.get()` reads and deserializes the value from the storage. 
        self.metadata.get().unwrap()
    }
}
```

## Compile smaller binaries

When compiling a contract make sure to pass flag `-C link-arg=-s` to the rust compiler:

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
```

Here is the parameters we use for the most examples in `Cargo.toml`:

```toml
[profile.release]
codegen-units = 1
opt-level = "s"
lto = true
debug = false
panic = "abort"
overflow-checks = true
```

You may want to experiment with using `opt-level = "z"` instead of `opt-level = "s"` to see if generates a smaller binary.

## Use simulation testing

Simulation testing allows you to run tests for multiple contracts and cross-contract calls in a simulated runtime environment.
Read more, [near-sdk-sim](https://github.com/near/near-sdk-rs/tree/master/near-sdk-sim)

## Appendix

### The traditional way of handling unique prefixes for persistent collections

Hardcoded prefixes in the constructor using a short one letter prefix that was converted to a vector of bytes.
When using nested collection, the prefix must be constructed manually.

```rust
#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            accounts: UnorderedMap::new(b"a"),
            tokens: LookupMap::new(b"t"),
            metadata: LazyOption::new(b"m"),
        }
    }

    fn get_tokens(&self, account_id: &AccountId) -> UnorderedSet<String> {
        let tokens = self.accounts.get(account_id).unwrap_or_else(|| {
            // Constructing a unique prefix for a nested UnorderedSet.
            let mut prefix = Vec::with_capacity(33);
            // Adding unique prefix.
            prefix.push(b's');
            // Adding the hash of the account_id (key of the outer map) to the prefix.
            // This is needed to differentiate across accounts.
            prefix.extend(env::sha256(account_id.as_bytes()));
            UnorderedSet::new(prefix)
        });
        tokens
    }
}
```
