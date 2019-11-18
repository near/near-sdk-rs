/*! A library for writing Rust smart contracts for Near Protocol blockchain.

This library provides both high-level and low-level API. You can use high-level API to write compact
an easy to read smart contracts, and you can use low-level API to write highly-optimized smart
contracts with the `*.wasm` size down to 400 bytes.

# Syntax
The following is a smart contract with two methods. `get_status` is a view method that can be called
without a transaction.

```
# use std::collections::HashMap;
# use near_bindgen::{near_bindgen, env};
# use borsh::{BorshSerialize, BorshDeserialize};
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

# Features

* **High-level strongly-typed API for asynchronous cross-contract calls.** You can decorate a trait
  representing the external contract with `#[ext_contract]` macro and then call its methods
  asyncrhonously:

```
# use near_bindgen::{ext_contract, near_bindgen, env};
# use borsh::{BorshDeserialize, BorshSerialize};
#[ext_contract]
pub trait Messenger {
    fn leave_message(&mut self, message: String);
    fn get_unread_messages(&self, sender_id: String) -> Vec<String>;
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct MyContract {}

#[near_bindgen]
impl MyContract {
    pub fn send(&self, receiver_id: String, message: String) {
            let prepaid_gas = env::prepaid_gas();
            let this_account = env::current_account_id();
            messenger::leave_message(message, &receiver_id, 0, prepaid_gas/3)
                .then(messenger::get_unread_messages(this_account, &receiver_id, 0, prepaid_gas/3));
        }
}
```

* **Creating transactions from inside the contract.** We can create any valid transaction from inside
the contract. For instance the following code issues two transactions: the first transaction creates
an account, transfers tokens to it, adds a public key, and deploys the contract; the second transaction
is scheduled to execute after the first one is finished, and it calls an initialization method.

```
# use near_bindgen::{ext_contract, near_bindgen, Promise, env};
# use borsh::{BorshDeserialize, BorshSerialize};
# const HOTEL_CODE: &[u8] = &[];
#[ext_contract]
pub trait Hotel {
    fn init(hotel_id: u64);
}

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct HotelFactory {}

#[near_bindgen]
impl HotelFactory {
    /// Asynchronously deploy several hotels.
    pub fn deploy_hotels(&self, num_hotels: u64) {
        let tokens_per_hotel = env::account_balance()/(num_hotels as u128 + 1); // Leave some for this account.
        let gas_per_hotel = env::prepaid_gas()/(num_hotels as u64 + 1); // Leave some for this execution.
        for i in 0..num_hotels {
            let account_id = format!("hotel{}", i);
            Promise::new(account_id.clone())
                .create_account()
                .transfer(tokens_per_hotel)
                .add_full_access_key(env::signer_account_pk())
                .deploy_contract(HOTEL_CODE.to_vec())
                .then(hotel::init(i, &account_id, 0, gas_per_hotel));
        }
    }
}
```

* **Unit-testable contracts.** You don't need to run a node to test your smart contract.
```
# use std::collections::HashMap;
# use near_bindgen::{near_bindgen, VMContext, env, testing_env};
# use borsh::{BorshSerialize, BorshDeserialize};
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

testing_env!(VMContext{ signer_account_id: "bob_near".to_string(), prepaid_gas: 1_000_000, ..Default::default()});
let mut contract = StatusMessage::default();
contract.set_status("hello".to_string());
assert_eq!("hello".to_string(), contract.get_status("bob_near".to_string()).unwrap());
```

*/
pub use near_bindgen_macros::{callback_args, callback_args_vec, ext_contract, near_bindgen};

pub mod collections;
mod environment;
pub use environment::env;

mod promise;
pub use promise::{Promise, PromiseOrValue};

#[cfg(not(target_arch = "wasm32"))]
pub use environment::mocked_blockchain::MockedBlockchain;
pub use near_vm_logic::types::*;
#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::VMConfig;
#[cfg(not(target_arch = "wasm32"))]
pub use near_vm_logic::VMContext;

/// A convenience macro for initializing unit testing fixtures.
/// * `testing_env!(context)` initializes fixture with default fees and VM configurations;
/// * `testing_env!(context, vm_config, fees_config)` initializes fixture with custom fees and VM configurations.
///
/// See `VMContext`, `VMConfig`, and `RuntimeFeesConfig` for the available settings.
#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! testing_env {
    ($context:expr) => {
        testing_env!($context, Default::default(), Default::default())
    };
    ($context:expr, $vm_config:expr, $fees_config:expr) => {
        let storage = match near_bindgen::env::take_blockchain_interface() {
            Some(mut bi) => bi.as_mut_mocked_blockchain().unwrap().take_storage(),
            None => Default::default(),
        };

        near_bindgen::env::set_blockchain_interface(Box::new(near_bindgen::MockedBlockchain::new(
            $context,
            $vm_config,
            $fees_config,
            vec![],
            storage,
        )));
    };
}

pub use environment::blockchain_interface::BlockchainInterface;
