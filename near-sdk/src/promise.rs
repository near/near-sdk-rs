#[cfg(feature = "abi")]
use borsh::BorshSchema;
use std::cell::RefCell;
#[cfg(feature = "abi")]
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::io::{Error, Write};
use std::mem;
use std::num::NonZeroU128;
use std::rc::Rc;

use crate::env::migrate_to_allowance;
use crate::CryptoHash;
use crate::{AccountId, Gas, GasWeight, NearToken, PromiseIndex, PublicKey};
use near_sdk_macros::near;

/// Allow an access key to spend either an unlimited or limited amount of gas
// This wrapper prevents incorrect construction
#[derive(Clone, Copy)]
pub enum Allowance {
    Unlimited,
    Limited(NonZeroU128),
}

impl Allowance {
    pub fn unlimited() -> Allowance {
        Allowance::Unlimited
    }

    /// This will return an None if you try to pass a zero value balance
    pub fn limited(balance: NearToken) -> Option<Allowance> {
        NonZeroU128::new(balance.as_yoctonear()).map(Allowance::Limited)
    }
}

enum PromiseAction {
    CreateAccount,
    DeployContract {
        code: Vec<u8>,
    },
    FunctionCall {
        function_name: String,
        arguments: Vec<u8>,
        amount: NearToken,
        gas: Gas,
    },
    FunctionCallWeight {
        function_name: String,
        arguments: Vec<u8>,
        amount: NearToken,
        gas: Gas,
        weight: GasWeight,
    },
    Transfer {
        amount: NearToken,
    },
    Stake {
        amount: NearToken,
        public_key: PublicKey,
    },
    AddFullAccessKey {
        public_key: PublicKey,
        nonce: u64,
    },
    AddAccessKey {
        public_key: PublicKey,
        allowance: Allowance,
        receiver_id: AccountId,
        function_names: String,
        nonce: u64,
    },
    DeleteKey {
        public_key: PublicKey,
    },
    DeleteAccount {
        beneficiary_id: AccountId,
    },
    #[cfg(feature = "global-contracts")]
    DeployGlobalContract {
        code: Vec<u8>,
    },
    #[cfg(feature = "global-contracts")]
    DeployGlobalContractByAccountId {
        code: Vec<u8>,
    },
    #[cfg(feature = "global-contracts")]
    UseGlobalContract {
        code_hash: CryptoHash,
    },
    #[cfg(feature = "global-contracts")]
    UseGlobalContractByAccountId {
        account_id: AccountId,
    },
    #[cfg(feature = "deterministic-account-ids")]
    DeterministicStateInit {
        state_init: crate::state_init::StateInit,
        deposit: NearToken,
    },
}

impl PromiseAction {
    pub fn add(self, promise_index: PromiseIndex) {
        use PromiseAction::*;
        match self {
            CreateAccount => crate::env::promise_batch_action_create_account(promise_index),
            DeployContract { code } => {
                crate::env::promise_batch_action_deploy_contract(promise_index, &code)
            }
            FunctionCall { function_name, arguments, amount, gas } => {
                crate::env::promise_batch_action_function_call(
                    promise_index,
                    function_name,
                    arguments,
                    amount,
                    gas,
                )
            }
            FunctionCallWeight { function_name, arguments, amount, gas, weight } => {
                crate::env::promise_batch_action_function_call_weight(
                    promise_index,
                    function_name,
                    arguments,
                    amount,
                    gas,
                    GasWeight(weight.0),
                )
            }
            Transfer { amount } => crate::env::promise_batch_action_transfer(promise_index, amount),
            Stake { amount, public_key } => {
                crate::env::promise_batch_action_stake(promise_index, amount, &public_key)
            }
            AddFullAccessKey { public_key, nonce } => {
                crate::env::promise_batch_action_add_key_with_full_access(
                    promise_index,
                    &public_key,
                    nonce,
                )
            }
            AddAccessKey { public_key, allowance, receiver_id, function_names, nonce } => {
                crate::env::promise_batch_action_add_key_allowance_with_function_call(
                    promise_index,
                    &public_key,
                    nonce,
                    allowance,
                    receiver_id,
                    function_names,
                )
            }
            DeleteKey { public_key } => {
                crate::env::promise_batch_action_delete_key(promise_index, &public_key)
            }
            DeleteAccount { beneficiary_id } => {
                crate::env::promise_batch_action_delete_account(promise_index, beneficiary_id)
            }
            #[cfg(feature = "global-contracts")]
            DeployGlobalContract { code } => {
                crate::env::promise_batch_action_deploy_global_contract(promise_index, code)
            }
            #[cfg(feature = "global-contracts")]
            DeployGlobalContractByAccountId { code } => {
                crate::env::promise_batch_action_deploy_global_contract_by_account_id(
                    promise_index,
                    &code,
                )
            }
            #[cfg(feature = "global-contracts")]
            UseGlobalContract { code_hash } => {
                crate::env::promise_batch_action_use_global_contract(promise_index, &code_hash)
            }
            #[cfg(feature = "global-contracts")]
            UseGlobalContractByAccountId { account_id } => {
                crate::env::promise_batch_action_use_global_contract_by_account_id(
                    promise_index,
                    account_id,
                )
            }
            #[cfg(feature = "deterministic-account-ids")]
            DeterministicStateInit {
                state_init: crate::state_init::StateInit::V1(state_init),
                deposit,
            } => {
                use crate::GlobalContractId;

                let action_index = match &state_init.code {
                    GlobalContractId::CodeHash(code_hash) => {
                        crate::env::promise_batch_action_state_init(
                            promise_index,
                            code_hash.into(),
                            deposit,
                        )
                    }
                    GlobalContractId::AccountId(account_id) => {
                        crate::env::promise_batch_action_state_init_by_account_id(
                            promise_index,
                            account_id,
                            deposit,
                        )
                    }
                };
                for (key, value) in &state_init.data {
                    crate::env::set_state_init_data_entry(promise_index, action_index, key, value);
                }
            }
        }
    }
}

enum PromiseSingleSubtype {
    Regular {
        account_id: AccountId,
        after: Option<Rc<RefCell<Promise>>>,
        /// Promise index that is computed only once.
        promise_index: Option<PromiseIndex>,
    },
    Yielded(PromiseIndex),
}

struct PromiseSingle {
    pub subtype: PromiseSingleSubtype,
    pub actions: Vec<PromiseAction>,
}

impl PromiseSingle {
    pub const fn new(subtype: PromiseSingleSubtype) -> Self {
        Self { subtype, actions: Vec::new() }
    }

    pub fn construct_recursively(&mut self) -> PromiseIndex {
        let promise_index = match &mut self.subtype {
            PromiseSingleSubtype::Regular { account_id, after, promise_index } => *promise_index
                .get_or_insert_with(|| {
                    if let Some(after) =
                        after.as_mut().and_then(|p| p.borrow_mut().construct_recursively())
                    {
                        crate::env::promise_batch_then(after, account_id)
                    } else {
                        crate::env::promise_batch_create(account_id)
                    }
                }),
            PromiseSingleSubtype::Yielded(promise_index) => *promise_index,
        };

        for action in mem::take(&mut self.actions) {
            action.add(promise_index);
        }

        promise_index
    }
}

pub struct PromiseJoint {
    pub promises: VecDeque<Promise>,
    /// Promise index that is computed only once.
    pub promise_index: Option<PromiseIndex>,
}

impl PromiseJoint {
    pub fn construct_recursively(&mut self) -> Option<PromiseIndex> {
        if self.promise_index.is_none() {
            let mut promises = mem::take(&mut self.promises);
            if promises.is_empty() {
                return None;
            }
            self.promise_index = Some(crate::env::promise_and(
                &promises.iter_mut().filter_map(Promise::construct_recursively).collect::<Vec<_>>(),
            ));
        }
        self.promise_index
    }
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
/// # use near_sdk::{ext_contract, near, AccountId, Promise, Gas};
/// #[ext_contract]
/// pub trait ContractB {
///     fn b(&mut self);
/// }
///
/// #[near(contract_state)]
/// #[derive(Default)]
/// struct ContractA {}
///
/// #[near]
/// impl ContractA {
///     pub fn a(&self) -> Promise {
///         contract_b::ext("bob_near".parse::<AccountId>().unwrap()).b()
///     }
/// }
/// ```
///
/// * When they need to create a transaction with one or many actions, e.g. the following code
///   schedules a transaction that creates an account, transfers tokens, and assigns a public key:
///
/// ```no_run
/// # use near_sdk::{AccountId, Promise, env, test_utils::VMContextBuilder, testing_env, Gas, NearToken};
/// # testing_env!(VMContextBuilder::new().signer_account_id("bob_near".parse().unwrap())
/// #               .account_balance(NearToken::from_yoctonear(1000)).prepaid_gas(Gas::from_gas(1_000_000)).build());
/// Promise::new("bob_near".parse::<AccountId>().unwrap())
///   .create_account()
///   .transfer(NearToken::from_yoctonear(1000))
///   .add_full_access_key(env::signer_account_pk());
/// ```
///
/// More information about promises in [NEAR documentation](https://docs.near.org/build/smart-contracts/anatomy/crosscontract#promises)
#[must_use = "return or detach explicitly via `.detach()`"]
pub struct Promise {
    subtype: PromiseSubtype,
    should_return: RefCell<bool>,
}

/// Until we implement strongly typed promises we serialize them as unit struct.
#[cfg(feature = "abi")]
impl BorshSchema for Promise {
    fn add_definitions_recursively(
        definitions: &mut BTreeMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        <()>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        <()>::declaration()
    }
}

enum PromiseSubtype {
    Single(PromiseSingle),
    Joint(PromiseJoint),
}

impl Promise {
    /// Create a promise that acts on the given account.
    /// Uses low-level [`crate::env::promise_batch_create`]
    pub fn new(account_id: impl Into<AccountId>) -> Self {
        Self::new_with_subtype(PromiseSubtype::Single(PromiseSingle::new(
            PromiseSingleSubtype::Regular {
                account_id: account_id.into(),
                after: None,
                promise_index: None,
            },
        )))
    }

    const fn new_with_subtype(subtype: PromiseSubtype) -> Self {
        Self { subtype, should_return: RefCell::new(false) }
    }

    /// Create a yielded promise that suspends execution until resumed.
    ///
    /// Returns a tuple of `(Promise, YieldId)` where:
    /// - The `Promise` represents the yielded execution that will call `function_name` when resumed
    /// - The `YieldId` can be stored and used later to resume the promise with [`YieldId::resume`]
    ///
    /// # Arguments
    /// * `function_name` - The callback function to invoke when the promise is resumed
    /// * `arguments` - Arguments to pass to the callback function
    /// * `gas` - Base gas to allocate for the callback
    /// * `weight` - Gas weight for distributing remaining gas
    ///
    /// # Important
    /// Yielded promises have a restriction:
    /// - **Cannot be used as continuations**: Using a yielded promise in `other.then(yielded)` will panic.
    ///   Yielded promises must be first in the chain: `yielded.then(other)` is valid.
    ///
    /// # Example
    /// ```ignore
    /// use near_sdk::{Promise, Gas, GasWeight};
    ///
    /// // Create a yielded promise
    /// let (promise, yield_id) = Promise::new_yield(
    ///     "on_data_received",
    ///     vec![],
    ///     Gas::from_tgas(10),
    ///     GasWeight(1),
    /// );
    ///
    /// // Chain another promise after the yielded one (valid)
    /// promise.then(Promise::new("other.near".parse().unwrap()).create_account());
    ///
    /// // Store yield_id to resume later from another transaction
    /// ```
    ///
    /// Uses low-level [`crate::env::promise_yield_create`]
    pub fn new_yield(
        function_name: impl AsRef<str>,
        arguments: impl AsRef<[u8]>,
        gas: Gas,
        weight: GasWeight,
    ) -> (Self, YieldId) {
        let (promise_index, yield_id) =
            crate::env::promise_yield_create_id(function_name, arguments, gas, weight);
        let promise = Self::new_with_subtype(PromiseSubtype::Single(PromiseSingle::new(
            PromiseSingleSubtype::Yielded(promise_index),
        )));
        (promise, yield_id)
    }

    fn add_action(mut self, action: PromiseAction) -> Self {
        match &mut self.subtype {
            PromiseSubtype::Single(x) => x.actions.push(action),
            PromiseSubtype::Joint(_) => {
                crate::env::panic_str("Cannot add action to a joint promise.")
            }
        }
        self
    }

    /// Create account on which this promise acts.
    /// Uses low-level [`crate::env::promise_batch_action_create_account`]
    pub fn create_account(self) -> Self {
        self.add_action(PromiseAction::CreateAccount)
    }

    /// Deploy a smart contract to the account on which this promise acts.
    /// Uses low-level [`crate::env::promise_batch_action_deploy_contract`]
    pub fn deploy_contract(self, code: impl Into<Vec<u8>>) -> Self {
        self.add_action(PromiseAction::DeployContract { code: code.into() })
    }

    #[cfg(feature = "global-contracts")]
    /// Deploy a global smart contract using the provided contract code.
    /// Uses low-level [`crate::env::promise_batch_action_deploy_global_contract`]
    ///
    /// # Examples
    /// ```no_run
    /// use near_sdk::{AccountId, Promise, NearToken};
    ///
    /// let code = vec![0u8; 100]; // Contract bytecode
    /// Promise::new("alice.near".parse::<AccountId>().unwrap())
    ///     .create_account()
    ///     .transfer(NearToken::from_yoctonear(1000))
    ///     .deploy_global_contract(code);
    /// ```
    pub fn deploy_global_contract(self, code: impl Into<Vec<u8>>) -> Self {
        self.add_action(PromiseAction::DeployGlobalContract { code: code.into() })
    }

    #[cfg(feature = "global-contracts")]
    /// Deploy a global smart contract, identifiable by the predecessor's account ID.
    /// Uses low-level [`crate::env::promise_batch_action_deploy_global_contract_by_account_id`]
    ///
    /// # Examples
    /// ```no_run
    /// use near_sdk::{AccountId, Promise, NearToken};
    ///
    /// let code = vec![0u8; 100]; // Contract bytecode
    /// Promise::new("alice.near".parse::<AccountId>().unwrap())
    ///     .create_account()
    ///     .transfer(NearToken::from_yoctonear(1000))
    ///     .deploy_global_contract_by_account_id(code);
    /// ```
    pub fn deploy_global_contract_by_account_id(self, code: impl Into<Vec<u8>>) -> Self {
        self.add_action(PromiseAction::DeployGlobalContractByAccountId { code: code.into() })
    }

    #[cfg(feature = "global-contracts")]
    /// Use an existing global contract by code hash.
    /// Uses low-level [`crate::env::promise_batch_action_use_global_contract`]
    ///
    /// # Examples
    /// ```no_run
    /// use near_sdk::{AccountId, Promise, NearToken};
    ///
    /// let code_hash = [0u8; 32]; // 32-byte hash (CryptoHash)
    /// Promise::new("alice.near".parse::<AccountId>().unwrap())
    ///     .create_account()
    ///     .transfer(NearToken::from_yoctonear(1000))
    ///     .use_global_contract(code_hash);
    /// ```
    pub fn use_global_contract(self, code_hash: impl Into<CryptoHash>) -> Self {
        self.add_action(PromiseAction::UseGlobalContract { code_hash: code_hash.into() })
    }

    #[cfg(feature = "global-contracts")]
    /// Use an existing global contract by referencing the account that deployed it.
    /// Uses low-level [`crate::env::promise_batch_action_use_global_contract_by_account_id`]
    ///
    /// # Examples
    /// ```no_run
    /// use near_sdk::{Promise, NearToken, AccountId};
    ///
    /// Promise::new("alice.near".parse::<AccountId>().unwrap())
    ///     .create_account()
    ///     .transfer(NearToken::from_yoctonear(1000))
    ///     .use_global_contract_by_account_id("deployer.near".parse().unwrap());
    /// ```
    pub fn use_global_contract_by_account_id(self, account_id: AccountId) -> Self {
        self.add_action(PromiseAction::UseGlobalContractByAccountId { account_id })
    }

    /// Creates a deterministic account with the given code, deposit, and data.
    #[cfg(feature = "deterministic-account-ids")]
    pub fn state_init(self, state_init: crate::state_init::StateInit, deposit: NearToken) -> Self {
        self.add_action(PromiseAction::DeterministicStateInit { state_init, deposit })
    }

    /// A low-level interface for making a function call to the account that this promise acts on.
    /// Uses low-level [`crate::env::promise_batch_action_function_call`]
    pub fn function_call(
        self,
        function_name: impl Into<String>,
        arguments: impl Into<Vec<u8>>,
        amount: NearToken,
        gas: Gas,
    ) -> Self {
        self.add_action(PromiseAction::FunctionCall {
            function_name: function_name.into(),
            arguments: arguments.into(),
            amount,
            gas,
        })
    }

    /// A low-level interface for making a function call to the account that this promise acts on.
    /// unlike [`Promise::function_call`], this function accepts a weight to use relative unused gas
    /// on this function call at the end of the scheduling method execution.
    /// Uses low-level [`crate::env::promise_batch_action_function_call_weight`]
    pub fn function_call_weight(
        self,
        function_name: impl Into<String>,
        arguments: impl Into<Vec<u8>>,
        amount: NearToken,
        gas: Gas,
        weight: GasWeight,
    ) -> Self {
        self.add_action(PromiseAction::FunctionCallWeight {
            function_name: function_name.into(),
            arguments: arguments.into(),
            amount,
            gas,
            weight,
        })
    }

    /// Transfer tokens to the account that this promise acts on.
    /// Uses low-level [`crate::env::promise_batch_action_transfer`]
    pub fn transfer(self, amount: NearToken) -> Self {
        self.add_action(PromiseAction::Transfer { amount })
    }

    /// Stake the account for the given amount of tokens using the given public key.
    /// Uses low-level [`crate::env::promise_batch_action_stake`]
    pub fn stake(self, amount: NearToken, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::Stake { amount, public_key })
    }

    /// Add full access key to the given account.
    /// Uses low-level [`crate::env::promise_batch_action_add_key_with_full_access`]
    pub fn add_full_access_key(self, public_key: PublicKey) -> Self {
        self.add_full_access_key_with_nonce(public_key, 0)
    }

    /// Add full access key to the given account with a provided nonce.
    /// Uses low-level [`crate::env::promise_batch_action_add_key_with_full_access`]
    pub fn add_full_access_key_with_nonce(self, public_key: PublicKey, nonce: u64) -> Self {
        self.add_action(PromiseAction::AddFullAccessKey { public_key, nonce })
    }

    /// Add an access key that is restricted to only calling a smart contract on some account using
    /// only a restricted set of methods. Here `function_names` is a comma separated list of methods,
    /// e.g. `"method_a,method_b"`.
    /// Uses low-level [`crate::env::promise_batch_action_add_key_allowance_with_function_call`]
    pub fn add_access_key_allowance(
        self,
        public_key: PublicKey,
        allowance: Allowance,
        receiver_id: AccountId,
        function_names: impl Into<String>,
    ) -> Self {
        self.add_access_key_allowance_with_nonce(
            public_key,
            allowance,
            receiver_id,
            function_names,
            0,
        )
    }

    #[deprecated(since = "5.0.0", note = "Use add_access_key_allowance instead")]
    pub fn add_access_key(
        self,
        public_key: PublicKey,
        allowance: NearToken,
        receiver_id: AccountId,
        function_names: impl Into<String>,
    ) -> Self {
        let allowance = migrate_to_allowance(allowance);
        self.add_access_key_allowance(public_key, allowance, receiver_id, function_names)
    }

    /// Add an access key with a provided nonce.
    /// Uses low-level [`crate::env::promise_batch_action_add_key_allowance_with_function_call`]
    pub fn add_access_key_allowance_with_nonce(
        self,
        public_key: PublicKey,
        allowance: Allowance,
        receiver_id: AccountId,
        function_names: impl Into<String>,
        nonce: u64,
    ) -> Self {
        self.add_action(PromiseAction::AddAccessKey {
            public_key,
            allowance,
            receiver_id,
            function_names: function_names.into(),
            nonce,
        })
    }

    #[deprecated(since = "5.0.0", note = "Use add_access_key_allowance_with_nonce instead")]
    pub fn add_access_key_with_nonce(
        self,
        public_key: PublicKey,
        allowance: NearToken,
        receiver_id: AccountId,
        function_names: impl Into<String>,
        nonce: u64,
    ) -> Self {
        let allowance = migrate_to_allowance(allowance);
        self.add_access_key_allowance_with_nonce(
            public_key,
            allowance,
            receiver_id,
            function_names,
            nonce,
        )
    }

    /// Delete access key from the given account.
    /// Uses low-level [`crate::env::promise_batch_action_delete_key`]
    pub fn delete_key(self, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::DeleteKey { public_key })
    }

    /// Delete the given account.
    /// Uses low-level [`crate::env::promise_batch_action_delete_account`]
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
    /// # use near_sdk::{AccountId, Promise, testing_env};
    /// let p1 = Promise::new("bob_near".parse::<AccountId>().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse::<AccountId>().unwrap()).create_account();
    /// let p3 = p1.and(p2);
    /// // p3.create_account();
    /// ```
    /// Uses low-level [`crate::env::promise_and`]
    pub fn and(mut self, mut other: Promise) -> Promise {
        match (&mut self.subtype, &mut other.subtype) {
            (PromiseSubtype::Joint(x), PromiseSubtype::Joint(o)) => {
                x.promises.append(&mut o.promises);
                self
            }
            (PromiseSubtype::Joint(x), _) => {
                x.promises.push_back(other);
                self
            }
            (_, PromiseSubtype::Joint(o)) => {
                o.promises.push_front(self);
                other
            }
            _ => Promise {
                subtype: PromiseSubtype::Joint(PromiseJoint {
                    promises: [self, other].into(),
                    promise_index: None,
                }),
                should_return: RefCell::new(false),
            },
        }
    }

    /// Schedules execution of another promise right after the current promise finish executing.
    ///
    /// In the following code `bob_near` and `dave_near` will be created concurrently. `carol_near`
    /// creation will wait for `bob_near` to be created, and `eva_near` will wait for both `carol_near`
    /// and `dave_near` to be created first.
    /// ```no_run
    /// # use near_sdk::{AccountId, Promise, VMContext, testing_env};
    /// let p1 = Promise::new("bob_near".parse::<AccountId>().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse::<AccountId>().unwrap()).create_account();
    /// let p3 = Promise::new("dave_near".parse::<AccountId>().unwrap()).create_account();
    /// let p4 = Promise::new("eva_near".parse::<AccountId>().unwrap()).create_account();
    /// p1.then(p2).and(p3).then(p4);
    /// ```
    /// Uses low-level [`crate::env::promise_batch_then`]
    pub fn then(self, other: Promise) -> Promise {
        Promise::then_impl(Rc::new(RefCell::new(self)), other)
    }

    /// Shared, private implementation between `then` and `then_concurrent`.
    ///
    /// This takes self as a reference counted object promise, to allow multiple
    /// dependencies pointing to the same promise without duplicating that
    /// promise.
    fn then_impl(this: Rc<RefCell<Self>>, mut other: Promise) -> Promise {
        match &mut other.subtype {
            PromiseSubtype::Single(x) => match &mut x.subtype {
                PromiseSingleSubtype::Regular { after, .. } => {
                    if after.replace(this).is_some() {
                        crate::env::panic_str(
                            "Cannot callback promise which is already scheduled after another",
                        );
                    }
                }
                PromiseSingleSubtype::Yielded(_) => {
                    crate::env::panic_str("Cannot callback yielded promise.")
                }
            },
            PromiseSubtype::Joint(_) => crate::env::panic_str("Cannot callback joint promise."),
        }
        other
    }

    /// Schedules execution of multiple concurrent promises right after the
    /// current promise finishes executing.
    ///
    /// This method will send the same return value as a data receipt to all
    /// following receipts.
    ///
    /// In the following code, `bob_near` is created first, `carol_near` second,
    /// and finally `dave_near` and `eva_near` are created concurrently.
    ///
    /// ```no_run
    /// # use near_sdk::{AccountId, Promise};
    /// let p1 = Promise::new("bob_near".parse::<AccountId>().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse::<AccountId>().unwrap()).create_account();
    /// let p3 = Promise::new("dave_near".parse::<AccountId>().unwrap()).create_account();
    /// let p4 = Promise::new("eva_near".parse::<AccountId>().unwrap()).create_account();
    /// p1.then(p2).then_concurrent(vec![p3, p4]);
    /// ```
    ///
    /// The returned [`ConcurrentPromises`] allows chaining more promises.
    ///
    /// In the following code, `bob_near` is created first, next `carol_near`
    /// and `dave_near` are created concurrently, and finally `eva_near` is
    /// created after all others have been created.
    ///
    /// ```no_run
    /// # use near_sdk::{AccountId, Promise};
    /// let p1 = Promise::new("bob_near".parse::<AccountId>().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse::<AccountId>().unwrap()).create_account();
    /// let p3 = Promise::new("dave_near".parse::<AccountId>().unwrap()).create_account();
    /// let p4 = Promise::new("eva_near".parse::<AccountId>().unwrap()).create_account();
    /// p1.then_concurrent(vec![p2, p3]).join().then(p4);
    /// ```
    pub fn then_concurrent(
        self,
        promises: impl IntoIterator<Item = Promise>,
    ) -> ConcurrentPromises {
        let this = Rc::new(RefCell::new(self));
        let mapped_promises =
            promises.into_iter().map(|other| Promise::then_impl(Rc::clone(&this), other)).collect();
        ConcurrentPromises { promises: mapped_promises }
    }

    /// A specialized, relatively low-level API method. Allows to mark the given promise as the one
    /// that should be considered as a return value.
    ///
    /// In the below code `a1` and `a2` functions are equivalent.
    /// ```
    /// # use near_sdk::{ext_contract, Gas, near, Promise, AccountId};
    /// #[ext_contract]
    /// pub trait ContractB {
    ///     fn b(&mut self);
    /// }
    ///
    /// #[near(contract_state)]
    /// #[derive(Default)]
    /// struct ContractA {}
    ///
    /// #[near]
    /// impl ContractA {
    ///     pub fn a1(&self) {
    ///        contract_b::ext("bob_near".parse::<AccountId>().unwrap()).b().as_return();
    ///     }
    ///
    ///     pub fn a2(&self) -> Promise {
    ///        contract_b::ext("bob_near".parse::<AccountId>().unwrap()).b()
    ///     }
    /// }
    /// ```
    /// Makes the promise to use low-level [`crate::env::promise_return`].
    #[allow(clippy::wrong_self_convention)]
    pub fn as_return(self) -> Self {
        *self.should_return.borrow_mut() = true;
        self
    }

    fn construct_recursively(&mut self) -> Option<PromiseIndex> {
        let res = match &mut self.subtype {
            PromiseSubtype::Single(x) => x.construct_recursively(),
            PromiseSubtype::Joint(x) => x.construct_recursively()?,
        };
        if *self.should_return.borrow() {
            crate::env::promise_return(res);
        }
        Some(res)
    }

    /// Explicitly detach given promise
    #[inline]
    pub fn detach(self) {}
}

impl Drop for Promise {
    fn drop(&mut self) {
        self.construct_recursively();
    }
}

impl serde::Serialize for Promise {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        *self.should_return.borrow_mut() = true;
        serializer.serialize_unit()
    }
}

impl borsh::BorshSerialize for Promise {
    fn serialize<W: Write>(&self, _writer: &mut W) -> Result<(), Error> {
        *self.should_return.borrow_mut() = true;

        // Intentionally no bytes written for the promise, the return value from the promise
        // will be considered as the return value from the contract call.
        Ok(())
    }
}

#[cfg(feature = "abi")]
impl schemars::JsonSchema for Promise {
    fn schema_name() -> String {
        "Promise".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Since promises are untyped, for now we represent Promise results with the schema
        // `true` which matches everything (i.e. always passes validation)
        schemars::schema::Schema::Bool(true)
    }
}

/// A unique identifier for a yielded promise that can be used to resume execution.
///
/// `YieldId` is returned by [`Promise::new_yield`] and can be stored or passed to another
/// transaction to resume the yielded promise with data.
///
/// # Example
/// ```ignore
/// // In the first transaction, create a yielded promise
/// let (promise, yield_id) = Promise::new_yield(
///     "on_resume",
///     vec![],
///     Gas::from_tgas(5),
///     GasWeight(1),
/// );
/// // Store yield_id somewhere (e.g., contract state)
///
/// // In a later transaction, resume with data
/// yield_id.resume(b"result data");
/// ```
#[near(inside_nearsdk, serializers = [json, borsh])]
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct YieldId(
    #[serde_as(as = "::serde_with::base64::Base64")]
    #[cfg_attr(feature = "abi", schemars(with = "String"))]
    pub(crate) CryptoHash,
);

/// Error returned when attempting to resume a yielded promise that was not found.
///
/// This occurs when:
/// - The promise was already resumed
/// - The promise timed out
/// - The `YieldId` is invalid
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ResumeError;

impl std::fmt::Display for ResumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to resume yielded promise: not found or already resumed")
    }
}

impl std::error::Error for ResumeError {}

impl YieldId {
    /// Resume the yielded promise with the provided data.
    ///
    /// Returns `Ok(())` if the promise was successfully resumed, or `Err(ResumeError)` if no promise
    /// with this `YieldId` was found (e.g., if it was already resumed or timed out).
    ///
    /// Uses low-level [`crate::env::promise_yield_resume`]
    pub fn resume(self, data: impl AsRef<[u8]>) -> Result<(), ResumeError> {
        if crate::env::promise_yield_resume(&self.0, data) {
            Ok(())
        } else {
            Err(ResumeError)
        }
    }
}

/// When the method can return either a promise or a value, it can be called with `PromiseOrValue::Promise`
/// or `PromiseOrValue::Value` to specify which one should be returned.
/// # Example
/// ```no_run
/// # use near_sdk::{ext_contract, near, Gas, PromiseOrValue, AccountId};
/// #[ext_contract]
/// pub trait ContractA {
///     fn a(&mut self);
/// }
///
/// let value = Some(true);
/// let val: PromiseOrValue<bool> = if let Some(value) = value {
///     PromiseOrValue::Value(value)
/// } else {
///     contract_a::ext("bob_near".parse::<AccountId>().unwrap()).a().into()
/// };
/// ```
#[must_use = "return or detach explicitly via `.detach()`"]
#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum PromiseOrValue<T> {
    Promise(Promise),
    Value(T),
}

impl<T> PromiseOrValue<T> {
    /// Explicitly detach if it was a promise
    #[inline]
    pub fn detach(self) {}
}

#[cfg(feature = "abi")]
impl<T> BorshSchema for PromiseOrValue<T>
where
    T: BorshSchema,
{
    fn add_definitions_recursively(
        definitions: &mut BTreeMap<borsh::schema::Declaration, borsh::schema::Definition>,
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

#[cfg(feature = "abi")]
impl<T: schemars::JsonSchema> schemars::JsonSchema for PromiseOrValue<T> {
    fn schema_name() -> String {
        format!("PromiseOrValue{}", T::schema_name())
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        T::json_schema(gen)
    }
}

/// A list of promises that are executed concurrently.
///
/// This is the return type of [`Promise::then_concurrent`] and it wraps the
/// contained promises for a more convenient syntax when chaining calls.
///
/// Use [`ConcurrentPromises::join`] to create a new promises that waits for all
/// concurrent promises and takes all their return values as inputs.
///
/// Use [`ConcurrentPromises::split_off`] to divide the list of promises into
/// subgroups that can be joined independently.
#[must_use = "return or detach explicitly via `.detach()`"]
pub struct ConcurrentPromises {
    promises: Vec<Promise>,
}

impl ConcurrentPromises {
    /// Create a new promises that waits for all contained concurrent promises.
    ///
    /// The returned promise is a [`Promise::and`] combination of all contained
    /// promises. Chain it with a [`Promise::then`] to wait for them to finish
    /// and receive all their outputs as inputs in a following function call.
    pub fn join(self) -> Promise {
        self.promises
            .into_iter()
            .reduce(|left, right| left.and(right))
            .expect("cannot join empty concurrent promises")
    }

    /// Splits the contained list of promises into two at the given index,
    /// returning a new [`ConcurrentPromises`] containing the elements in the
    /// range [at, len).
    ///
    /// After the call, the original [`ConcurrentPromises`] will be left
    /// containing the elements [0, at).
    pub fn split_off(&mut self, at: usize) -> ConcurrentPromises {
        let right_side = self.promises.split_off(at);
        ConcurrentPromises { promises: right_side }
    }

    /// Explicitly detach given promises
    #[inline]
    pub fn detach(self) {}
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use crate::mock::MockAction;
    use crate::test_utils::get_created_receipts;
    use crate::test_utils::test_env::{alice, bob};
    use crate::{
        test_utils::VMContextBuilder, testing_env, AccountId, Allowance, NearToken, Promise,
        PublicKey,
    };

    fn pk() -> PublicKey {
        "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse().unwrap()
    }

    fn get_actions() -> std::vec::IntoIter<MockAction> {
        let receipts = get_created_receipts();
        let first_receipt = receipts.into_iter().next().unwrap();
        first_receipt.actions.into_iter()
    }

    fn has_add_key_with_full_access(public_key: PublicKey, nonce: Option<u64>) -> bool {
        let public_key = near_crypto::PublicKey::try_from(public_key).unwrap();
        get_actions().any(|el| {
            matches!(
                el,
                MockAction::AddKeyWithFullAccess { public_key: p, nonce: n, receipt_index: _, }
                if p == public_key
                    && (nonce.is_none() || Some(n) == nonce)
            )
        })
    }

    fn has_add_key_with_function_call(
        public_key: PublicKey,
        allowance: u128,
        receiver_id: AccountId,
        function_names: String,
        nonce: Option<u64>,
    ) -> bool {
        let public_key = near_crypto::PublicKey::try_from(public_key).unwrap();
        get_actions().any(|el| {
            matches!(
                el,
                MockAction::AddKeyWithFunctionCall {
                    public_key: p,
                    allowance: a,
                    receiver_id: r,
                    method_names,
                    nonce: n,
                    receipt_index: _,
                }
                if p == public_key
                    && a.unwrap() == NearToken::from_yoctonear(allowance)
                    && r == receiver_id
                    && method_names.clone() == function_names.split(',').collect::<Vec<_>>()
                    && (nonce.is_none() || Some(n) == nonce)
            )
        })
    }

    #[test]
    fn test_add_full_access_key() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();

        // Promise is only executed when dropped so we put it in its own scope to make sure receipts
        // are ready afterwards.
        {
            Promise::new(alice()).create_account().add_full_access_key(public_key.clone()).detach();
        }

        assert!(has_add_key_with_full_access(public_key, None));
    }

    #[test]
    fn test_add_full_access_key_with_nonce() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let nonce = 42;

        {
            Promise::new(alice())
                .create_account()
                .add_full_access_key_with_nonce(public_key.clone(), nonce)
                .detach();
        }

        assert!(has_add_key_with_full_access(public_key, Some(nonce)));
    }

    #[test]
    fn test_add_access_key_allowance() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let allowance = 100;
        let receiver_id = bob();
        let function_names = "method_a,method_b".to_string();

        {
            Promise::new(alice())
                .create_account()
                .add_access_key_allowance(
                    public_key.clone(),
                    Allowance::Limited(allowance.try_into().unwrap()),
                    receiver_id.clone(),
                    function_names.clone(),
                )
                .detach();
        }

        assert!(has_add_key_with_function_call(
            public_key,
            allowance,
            receiver_id,
            function_names,
            None
        ));
    }

    #[test]
    fn test_add_access_key() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let allowance = NearToken::from_yoctonear(100);
        let receiver_id = bob();
        let function_names = "method_a,method_b".to_string();

        {
            #[allow(deprecated)]
            Promise::new(alice())
                .create_account()
                .add_access_key(
                    public_key.clone(),
                    allowance,
                    receiver_id.clone(),
                    function_names.clone(),
                )
                .detach();
        }

        assert!(has_add_key_with_function_call(
            public_key,
            allowance.as_yoctonear(),
            receiver_id,
            function_names,
            None
        ));
    }

    #[test]
    fn test_add_access_key_allowance_with_nonce() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let allowance = 100;
        let receiver_id = bob();
        let function_names = "method_a,method_b".to_string();
        let nonce = 42;

        {
            Promise::new(alice())
                .create_account()
                .add_access_key_allowance_with_nonce(
                    public_key.clone(),
                    Allowance::Limited(allowance.try_into().unwrap()),
                    receiver_id.clone(),
                    function_names.clone(),
                    nonce,
                )
                .detach();
        }

        assert!(has_add_key_with_function_call(
            public_key,
            allowance,
            receiver_id,
            function_names,
            Some(nonce)
        ));
    }

    #[test]
    fn test_add_access_key_with_nonce() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let allowance = NearToken::from_yoctonear(100);
        let receiver_id = bob();
        let function_names = "method_a,method_b".to_string();
        let nonce = 42;

        {
            #[allow(deprecated)]
            Promise::new(alice())
                .create_account()
                .add_access_key_with_nonce(
                    public_key.clone(),
                    allowance,
                    receiver_id.clone(),
                    function_names.clone(),
                    nonce,
                )
                .detach();
        }

        assert!(has_add_key_with_function_call(
            public_key,
            allowance.as_yoctonear(),
            receiver_id,
            function_names,
            Some(nonce)
        ));
    }

    #[test]
    fn test_delete_key() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();

        {
            Promise::new(alice())
                .create_account()
                .add_full_access_key(public_key.clone())
                .delete_key(public_key.clone())
                .detach();
        }
        let public_key = near_crypto::PublicKey::try_from(public_key).unwrap();

        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::DeleteKey { public_key: p , receipt_index: _, } if p == public_key
            )
        });
        assert!(has_action);
    }

    #[cfg(feature = "global-contracts")]
    #[test]
    fn test_deploy_global_contract() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let code = vec![1, 2, 3, 4];

        {
            Promise::new(alice()).create_account().deploy_global_contract(code.clone()).detach();
        }

        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::DeployGlobalContract { code: c, receipt_index: _, mode: _ } if c == code
            )
        });
        assert!(has_action);
    }

    #[cfg(feature = "global-contracts")]
    #[test]
    fn test_deploy_global_contract_by_account_id() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let code = vec![5, 6, 7, 8];

        {
            Promise::new(alice())
                .create_account()
                .deploy_global_contract_by_account_id(code.clone())
                .detach();
        }

        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::DeployGlobalContract { code: c, receipt_index: _, mode: _ } if c == code
            )
        });
        assert!(has_action);
    }

    #[cfg(feature = "global-contracts")]
    #[test]
    fn test_use_global_contract() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let code_hash = [0u8; 32];

        {
            Promise::new(alice()).create_account().use_global_contract(code_hash).detach();
        }

        // Check if any UseGlobalContract action exists
        let has_action = get_actions().any(|el| matches!(el, MockAction::UseGlobalContract { .. }));
        assert!(has_action);
    }

    #[cfg(feature = "global-contracts")]
    #[test]
    fn test_use_global_contract_by_account_id() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let deployer = bob();

        {
            Promise::new(alice())
                .create_account()
                .use_global_contract_by_account_id(deployer.clone())
                .detach();
        }

        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::UseGlobalContract {
                    contract_id: near_primitives::action::GlobalContractIdentifier::AccountId(contract_id),
                    receipt_index: _
                }
                if contract_id == deployer
            )
        });
        assert!(has_action);
    }

    #[test]
    fn test_then() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());
        let sub_account_1: AccountId = "sub1.alice.near".parse().unwrap();

        {
            Promise::new(alice())
                .create_account()
                .then(Promise::new(sub_account_1).create_account())
                .detach();
        }

        let receipts = get_created_receipts();
        let main_account_creation = &receipts[0];
        let sub_creation = &receipts[1];

        assert!(
            main_account_creation.receipt_indices.is_empty(),
            "first receipt must not have dependencies"
        );
        assert_eq!(
            &sub_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
    }

    #[test]
    fn test_then_concurrent() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());
        let sub_account_1: AccountId = "sub1.alice.near".parse().unwrap();
        let sub_account_2: AccountId = "sub2.alice.near".parse().unwrap();

        {
            let p1 = Promise::new(sub_account_1.clone()).create_account();
            let p2 = Promise::new(sub_account_2.clone()).create_account();
            Promise::new(alice()).create_account().then_concurrent(vec![p1, p2]).detach();
        }

        let receipts = get_created_receipts();
        let main_account_creation = &receipts[0];

        let sub1_creation = &receipts[1];
        let sub2_creation = &receipts[2];

        // ensure we are looking at the right receipts
        assert_eq!(sub1_creation.receiver_id, sub_account_1);
        assert_eq!(sub2_creation.receiver_id, sub_account_2);

        // Check dependencies were created
        assert!(
            main_account_creation.receipt_indices.is_empty(),
            "first receipt must not have dependencies"
        );
        assert_eq!(
            &sub1_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
        assert_eq!(
            &sub2_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
    }

    #[test]
    fn test_then_concurrent_split_off_then() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());
        let sub_account_1: AccountId = "sub1.alice.near".parse().unwrap();
        let sub_account_2: AccountId = "sub2.alice.near".parse().unwrap();
        let sub_account_3: AccountId = "sub3.sub2.alice.near".parse().unwrap();

        {
            let p1 = Promise::new(sub_account_1.clone()).create_account();
            let p2 = Promise::new(sub_account_2.clone()).create_account();
            let p3 = Promise::new(sub_account_3.clone()).create_account();
            Promise::new(alice())
                .create_account()
                .then_concurrent(vec![p1, p2])
                .split_off(1)
                .join()
                .then(p3)
                .detach();
        }

        let receipts = get_created_receipts();
        let main_account_creation = &receipts[0];

        // recursive construction switches the order of receipts
        let sub1_creation = &receipts[3];
        let sub2_creation = &receipts[1];
        let sub3_creation = &receipts[2];

        // ensure we are looking at the right receipts
        assert_eq!(sub1_creation.receiver_id, sub_account_1);
        assert_eq!(sub2_creation.receiver_id, sub_account_2);
        assert_eq!(sub3_creation.receiver_id, sub_account_3);

        // Find receipt index to depend on
        let sub2_creation_index = sub2_creation.actions[0].receipt_index().unwrap();

        // Check dependencies were created
        assert!(
            main_account_creation.receipt_indices.is_empty(),
            "first receipt must not have dependencies"
        );
        assert_eq!(
            &sub1_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
        assert_eq!(
            &sub2_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
        assert_eq!(
            &sub3_creation.receipt_indices,
            &[sub2_creation_index],
            "then() must create dependency on sub2_creation"
        );
    }

    #[test]
    fn test_then_concurrent_twice() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());
        let sub_account_1: AccountId = "sub1.alice.near".parse().unwrap();
        let sub_account_2: AccountId = "sub2.alice.near".parse().unwrap();
        let sub_account_3: AccountId = "sub3.sub2.alice.near".parse().unwrap();
        let sub_account_4: AccountId = "sub4.sub2.alice.near".parse().unwrap();

        {
            let p1 = Promise::new(sub_account_1.clone()).create_account();
            let p2 = Promise::new(sub_account_2.clone()).create_account();
            let p3 = Promise::new(sub_account_3.clone()).create_account();
            let p4 = Promise::new(sub_account_4.clone()).create_account();
            Promise::new(alice())
                .create_account()
                .then_concurrent(vec![p1, p2])
                .join()
                .then_concurrent(vec![p3, p4])
                .detach();
        }

        let receipts = get_created_receipts();
        let main_account_creation = &receipts[0];
        // recursive construction switches the order of receipts
        let sub1_creation = &receipts[1];
        let sub2_creation = &receipts[2];
        let sub3_creation = &receipts[3];
        let sub4_creation = &receipts[4];

        // ensure we are looking at the right receipts
        assert_eq!(sub1_creation.receiver_id, sub_account_1);
        assert_eq!(sub2_creation.receiver_id, sub_account_2);
        assert_eq!(sub3_creation.receiver_id, sub_account_3);
        assert_eq!(sub4_creation.receiver_id, sub_account_4);

        // Find receipt indices to depend on
        let sub1_creation_index = sub1_creation.actions[0].receipt_index().unwrap();
        let sub2_creation_index = sub2_creation.actions[0].receipt_index().unwrap();

        // Check dependencies were created
        assert!(
            main_account_creation.receipt_indices.is_empty(),
            "first receipt must not have dependencies"
        );
        assert_eq!(
            &sub1_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
        assert_eq!(
            &sub2_creation.receipt_indices,
            &[0],
            "then_concurrent() must create dependency on receipt 0"
        );
        assert_eq!(
            &sub3_creation.receipt_indices,
            &[sub1_creation_index, sub2_creation_index],
            "then_concurrent() must create dependency on sub1_creation_index + sub2_creation"
        );
        assert_eq!(
            &sub4_creation.receipt_indices,
            &[sub1_creation_index, sub2_creation_index],
            "then_concurrent() must create dependency on sub1_creation_index + sub2_creation"
        );
    }

    #[test]
    #[should_panic(expected = "Cannot callback yielded promise.")]
    fn test_yielded_promise_cannot_be_continuation() {
        use crate::{Gas, GasWeight};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let (yielded, _yield_id) =
            Promise::new_yield("callback", vec![], Gas::from_tgas(5), GasWeight(1));

        let regular = Promise::new(alice()).create_account();

        // This should panic - yielded promises cannot be used as continuations
        // i.e., they cannot appear on the right side of .then()
        regular.then(yielded).detach();
    }

    #[test]
    fn test_new_yield_creates_promise_and_yield_id() {
        use crate::{Gas, GasWeight};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        // Create a yielded promise
        let (_promise, yield_id) =
            Promise::new_yield("on_callback", b"test_args", Gas::from_tgas(10), GasWeight(1));

        // Verify yield_id is a valid 32-byte CryptoHash
        let yield_id_bytes: [u8; 32] = yield_id.0;
        // The mock generates a random yield ID, so just check it's not all zeros
        assert!(yield_id_bytes.iter().any(|&b| b != 0), "YieldId should not be all zeros");
    }

    #[test]
    fn test_yield_id_is_unique() {
        use crate::{Gas, GasWeight};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        // Create multiple yielded promises and verify they get unique YieldIds
        let (_promise1, yield_id1) =
            Promise::new_yield("on_callback1", vec![], Gas::from_tgas(5), GasWeight(1));
        let (_promise2, yield_id2) =
            Promise::new_yield("on_callback2", vec![], Gas::from_tgas(5), GasWeight(1));

        // The two yield IDs should be different
        assert_ne!(yield_id1, yield_id2, "Two yielded promises should have different YieldIds");
    }

    #[test]
    #[ignore]
    //TODO: currently mock does not support yielded promises
    // uncomment after https://github.com/near/nearcore/pull/14792 is released
    fn test_yielded_promise_can_chain_then() {
        use crate::{Gas, GasWeight};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        // Create a yielded promise and chain another promise after it
        {
            let (yielded, _yield_id) =
                Promise::new_yield("on_resume", vec![], Gas::from_tgas(5), GasWeight(1));

            // This should not panic - yielded promises CAN be first in a chain
            yielded.then(Promise::new(bob()).transfer(NearToken::from_yoctonear(1000))).detach();
        }

        // Verify the chained promise was created (receipts are created on drop)
        let receipts = get_created_receipts();
        // The chained promise creates a receipt for bob()
        let bob_receipt = receipts
            .iter()
            .find(|r| r.receiver_id == bob())
            .expect("Should have created a receipt for bob()");

        // Verify the exact transfer action in the chained receipt
        assert_eq!(bob_receipt.actions.len(), 1, "bob() receipt should have exactly 1 action");
        let receipt_index = bob_receipt.actions[0].receipt_index().unwrap();
        assert_eq!(
            bob_receipt.actions[0],
            MockAction::Transfer { receipt_index, deposit: NearToken::from_yoctonear(1000) }
        );
    }
}
