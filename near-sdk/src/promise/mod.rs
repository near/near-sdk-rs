mod queue;

use borsh::BorshSchema;
use std::collections::HashMap;
use std::io::{Error, Write};

use self::queue::{queue_promise_event, PromiseQueueEvent};
pub use self::queue::{schedule_queued_promises, QueueIndex};
use crate::{AccountId, Balance, Gas, PublicKey};

pub enum PromiseAction {
    CreateAccount,
    DeployContract {
        code: Vec<u8>,
    },
    FunctionCall {
        method_name: String,
        arguments: Vec<u8>,
        amount: Balance,
        gas: Option<Gas>,
    },
    Transfer {
        amount: Balance,
    },
    Stake {
        amount: Balance,
        public_key: PublicKey,
    },
    AddFullAccessKey {
        public_key: PublicKey,
        nonce: u64,
    },
    AddAccessKey {
        public_key: PublicKey,
        allowance: Balance,
        receiver_id: AccountId,
        method_names: String,
        nonce: u64,
    },
    DeleteKey {
        public_key: PublicKey,
    },
    DeleteAccount {
        beneficiary_id: AccountId,
    },
}

/// A structure representing a result of the scheduled execution on another contract.
///
/// Smart contract developers will explicitly use `Promise` in two situations:
/// * When they need to return `Promise`.
///
///   In the following code if someone calls method `ContractA::a` they will internally cause an
///   execution of method `ContractB::b` of `bob_near` account, and the return value of `ContractA::a`
///   will be what `ContractB::b` returned.
/// ```no_run
/// # use near_sdk::{ext_contract, near_bindgen, Promise, Gas};
/// # use borsh::{BorshDeserialize, BorshSerialize};
/// #[ext_contract]
/// pub trait ContractB {
///     fn b(&mut self);
/// }
///
/// #[near_bindgen]
/// #[derive(Default, BorshDeserialize, BorshSerialize)]
/// struct ContractA {}
///
/// #[near_bindgen]
/// impl ContractA {
///     pub fn a(&self) -> Promise {
///         contract_b::b("bob_near".parse().unwrap(), 0, Gas(1_000))
///     }
/// }
/// ```
///
/// * When they need to create a transaction with one or many actions, e.g. the following code
///   schedules a transaction that creates an account, transfers tokens, and assigns a public key:
///
/// ```no_run
/// # use near_sdk::{Promise, env, test_utils::VMContextBuilder, testing_env};
/// # testing_env!(VMContextBuilder::new().signer_account_id("bob_near".parse().unwrap())
/// #               .account_balance(1000).prepaid_gas(1_000_000.into()).build());
/// Promise::new("bob_near".parse().unwrap())
///   .create_account()
///   .transfer(1000)
///   .add_full_access_key(env::signer_account_pk());
/// ```
pub struct Promise {
    subtype: PromiseSubtype,
}

/// Until we implement strongly typed promises we serialize them as unit struct.
impl BorshSchema for Promise {
    fn add_definitions_recursively(
        definitions: &mut HashMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        <()>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        <()>::declaration()
    }
}

pub enum PromiseSubtype {
    Single(QueueIndex),
    Joint(QueueIndex),
}

impl PromiseSubtype {
    fn index(&self) -> QueueIndex {
        match self {
            Self::Single(x) => *x,
            Self::Joint(x) => *x,
        }
    }
}

impl Promise {
    /// Create a promise that acts on the given account.
    pub fn new(account_id: AccountId) -> Self {
        let index = queue_promise_event(PromiseQueueEvent::CreateBatch { account_id });
        Self { subtype: PromiseSubtype::Single(index) }
    }

    fn add_action(self, action: PromiseAction) -> Self {
        match &self.subtype {
            PromiseSubtype::Single(x) => {
                queue_promise_event(PromiseQueueEvent::Action { index: *x, action });
            }
            PromiseSubtype::Joint(_) => {
                crate::env::panic_str("Cannot add action to a joint promise.")
            }
        }
        self
    }

    /// Create account on which this promise acts.
    pub fn create_account(self) -> Self {
        self.add_action(PromiseAction::CreateAccount)
    }

    /// Deploy a smart contract to the account on which this promise acts.
    pub fn deploy_contract(self, code: Vec<u8>) -> Self {
        self.add_action(PromiseAction::DeployContract { code })
    }

    /// A low-level interface for making a function call to the account that this promise acts on.
    pub fn function_call(
        self,
        method_name: String,
        arguments: Vec<u8>,
        amount: Balance,
        gas: Gas,
    ) -> Self {
        self.add_action(PromiseAction::FunctionCall {
            method_name,
            arguments,
            amount,
            gas: Some(gas),
        })
    }

    /// Queue a function call which will be scheduled with the promise
    pub fn queued_function_call(
        self,
        method_name: String,
        arguments: Vec<u8>,
        amount: Balance,
        gas: Option<Gas>,
    ) -> Self {
        self.add_action(PromiseAction::FunctionCall { method_name, arguments, amount, gas })
    }

    /// Transfer tokens to the account that this promise acts on.
    pub fn transfer(self, amount: Balance) -> Self {
        self.add_action(PromiseAction::Transfer { amount })
    }

    /// Stake the account for the given amount of tokens using the given public key.
    pub fn stake(self, amount: Balance, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::Stake { amount, public_key })
    }

    /// Add full access key to the given account.
    pub fn add_full_access_key(self, public_key: PublicKey) -> Self {
        self.add_full_access_key_with_nonce(public_key, 0)
    }

    /// Add full access key to the given account with a provided nonce.
    pub fn add_full_access_key_with_nonce(self, public_key: PublicKey, nonce: u64) -> Self {
        self.add_action(PromiseAction::AddFullAccessKey { public_key, nonce })
    }

    /// Add an access key that is restricted to only calling a smart contract on some account using
    /// only a restricted set of methods. Here `method_names` is a comma separated list of methods,
    /// e.g. `"method_a,method_b".to_string()`.
    pub fn add_access_key(
        self,
        public_key: PublicKey,
        allowance: Balance,
        receiver_id: AccountId,
        method_names: String,
    ) -> Self {
        self.add_access_key_with_nonce(public_key, allowance, receiver_id, method_names, 0)
    }

    /// Add an access key with a provided nonce.
    pub fn add_access_key_with_nonce(
        self,
        public_key: PublicKey,
        allowance: Balance,
        receiver_id: AccountId,
        method_names: String,
        nonce: u64,
    ) -> Self {
        self.add_action(PromiseAction::AddAccessKey {
            public_key,
            allowance,
            receiver_id,
            method_names,
            nonce,
        })
    }

    /// Delete access key from the given account.
    pub fn delete_key(self, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::DeleteKey { public_key })
    }

    /// Delete the given account.
    pub fn delete_account(self, beneficiary_id: AccountId) -> Self {
        self.add_action(PromiseAction::DeleteAccount { beneficiary_id })
    }

    /// Merge this promise with another promise, so that we can schedule execution of another
    /// smart contract right after all merged promises finish.
    ///
    /// Note, once the promises are merged it is not possible to add actions to them, e.g. the
    /// following code will panic during the execution of the smart contract:
    ///
    /// ```no_run
    /// # use near_sdk::{Promise, testing_env};
    /// let p1 = Promise::new("bob_near".parse().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse().unwrap()).create_account();
    /// let p3 = p1.and(p2);
    /// // p3.create_account();
    /// ```
    pub fn and(self, other: Promise) -> Promise {
        let idx = queue_promise_event(PromiseQueueEvent::BatchAnd {
            promise_a: self.subtype.index(),
            promise_b: other.subtype.index(),
        });
        Promise { subtype: PromiseSubtype::Joint(idx) }
    }

    /// Schedules execution of another promise right after the current promise finish executing.
    ///
    /// In the following code `bob_near` and `dave_near` will be created concurrently. `carol_near`
    /// creation will wait for `bob_near` to be created, and `eva_near` will wait for both `carol_near`
    /// and `dave_near` to be created first.
    /// ```no_run
    /// # use near_sdk::{Promise, VMContext, testing_env};
    /// let p1 = Promise::new("bob_near".parse().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse().unwrap()).create_account();
    /// let p3 = Promise::new("dave_near".parse().unwrap()).create_account();
    /// let p4 = Promise::new("eva_near".parse().unwrap()).create_account();
    /// p1.then(p2).and(p3).then(p4);
    /// ```
    pub fn then(self, other: Promise) -> Promise {
        match &other.subtype {
            PromiseSubtype::Single(x) => {
                queue::upgrade_to_then(self.subtype.index(), *x);
            }
            PromiseSubtype::Joint(_) => crate::env::panic_str("Cannot callback joint promise."),
        }

        other
    }

    /// A specialized, relatively low-level API method. Allows to mark the given promise as the one
    /// that should be considered as a return value.
    ///
    /// In the below code `a1` and `a2` functions are equivalent.
    /// ```
    /// # use near_sdk::{ext_contract, Gas, near_bindgen, Promise};
    /// # use borsh::{BorshDeserialize, BorshSerialize};
    /// #[ext_contract]
    /// pub trait ContractB {
    ///     fn b(&mut self);
    /// }
    ///
    /// #[near_bindgen]
    /// #[derive(Default, BorshDeserialize, BorshSerialize)]
    /// struct ContractA {}
    ///
    /// #[near_bindgen]
    /// impl ContractA {
    ///     pub fn a1(&self) {
    ///        contract_b::b("bob_near".parse().unwrap(), 0, Gas(1_000)).as_return();
    ///     }
    ///
    ///     pub fn a2(&self) -> Promise {
    ///        contract_b::b("bob_near".parse().unwrap(), 0, Gas(1_000))
    ///     }
    /// }
    /// ```
    #[allow(clippy::wrong_self_convention)]
    pub fn as_return(self) -> Self {
        // TODO handle duplicate should return, before would just be queued once
        queue_promise_event(PromiseQueueEvent::Return { index: self.subtype.index() });
        // *self.should_return.borrow_mut() = true;
        self
    }
}

impl serde::Serialize for Promise {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        queue_promise_event(PromiseQueueEvent::Return { index: self.subtype.index() });
        serializer.serialize_unit()
    }
}

impl borsh::BorshSerialize for Promise {
    fn serialize<W: Write>(&self, _writer: &mut W) -> Result<(), Error> {
        queue_promise_event(PromiseQueueEvent::Return { index: self.subtype.index() });

        // Intentionally no bytes written for the promise, the return value from the promise
        // will be considered as the return value from the contract call.
        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum PromiseOrValue<T> {
    Promise(Promise),
    Value(T),
}

impl<T> BorshSchema for PromiseOrValue<T>
where
    T: BorshSchema,
{
    fn add_definitions_recursively(
        definitions: &mut HashMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        T::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        T::declaration()
    }
}

impl<T> From<Promise> for PromiseOrValue<T> {
    fn from(promise: Promise) -> Self {
        PromiseOrValue::Promise(promise)
    }
}

impl<T: borsh::BorshSerialize> borsh::BorshSerialize for PromiseOrValue<T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            // Only actual value is serialized.
            PromiseOrValue::Value(x) => x.serialize(writer),
            // The promise is dropped to cause env::promise calls.
            PromiseOrValue::Promise(p) => p.serialize(writer),
        }
    }
}
