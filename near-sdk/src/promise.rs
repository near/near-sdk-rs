#[cfg(feature = "abi")]
use std::collections::BTreeMap;
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{Error, Write},
    mem,
    rc::Rc,
};

#[cfg(feature = "abi")]
use borsh::BorshSchema;
use near_sdk_macros::near;

use crate::{
    AccountId, CryptoHash, Gas, GasWeight, NearToken, PromiseIndex, PublicKey,
    env::migrate_to_allowance,
};

pub use near_sdk_core::allowance::Allowance;

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
    TransferToGasKey {
        public_key: PublicKey,
        amount: NearToken,
    },
    AddGasKeyFullAccess {
        public_key: PublicKey,
        num_nonces: u32,
    },
    AddGasKeyFunctionCall {
        public_key: PublicKey,
        num_nonces: u32,
        allowance: Allowance,
        receiver_id: AccountId,
        function_names: String,
    },
    DeployGlobalContract {
        code: Vec<u8>,
    },
    DeployGlobalContractByAccountId {
        code: Vec<u8>,
    },
    UseGlobalContract {
        code_hash: CryptoHash,
    },
    UseGlobalContractByAccountId {
        account_id: AccountId,
    },
    #[cfg(feature = "deterministic-account-ids")]
    DeterministicStateInit {
        state_init: crate::state_init::StateInit,
        deposit: NearToken,
    },
    UniversalStateInit {
        state_init: crate::universal_state_init::RawStateInit,
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
                    &function_name,
                    &arguments,
                    amount,
                    gas,
                )
            }
            FunctionCallWeight { function_name, arguments, amount, gas, weight } => {
                crate::env::promise_batch_action_function_call_weight(
                    promise_index,
                    &function_name,
                    &arguments,
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
                    &receiver_id,
                    &function_names,
                )
            }
            DeleteKey { public_key } => {
                crate::env::promise_batch_action_delete_key(promise_index, &public_key)
            }
            DeleteAccount { beneficiary_id } => {
                crate::env::promise_batch_action_delete_account(promise_index, &beneficiary_id)
            }
            TransferToGasKey { public_key, amount } => {
                crate::env::promise_batch_action_transfer_to_gas_key(
                    promise_index,
                    &public_key,
                    amount,
                )
            }
            AddGasKeyFullAccess { public_key, num_nonces } => {
                crate::env::promise_batch_action_add_gas_key_with_full_access(
                    promise_index,
                    &public_key,
                    num_nonces,
                )
            }
            AddGasKeyFunctionCall {
                public_key,
                num_nonces,
                allowance,
                receiver_id,
                function_names,
            } => crate::env::promise_batch_action_add_gas_key_with_function_call(
                promise_index,
                &public_key,
                num_nonces,
                allowance,
                &receiver_id,
                &function_names,
            ),
            DeployGlobalContract { code } => {
                crate::env::promise_batch_action_deploy_global_contract(promise_index, &code)
            }
            DeployGlobalContractByAccountId { code } => {
                crate::env::promise_batch_action_deploy_global_contract_by_account_id(
                    promise_index,
                    &code,
                )
            }
            UseGlobalContract { code_hash } => {
                crate::env::promise_batch_action_use_global_contract(promise_index, &code_hash)
            }
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
                            *code_hash,
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
            UniversalStateInit { state_init, deposit } => {
                crate::env::promise_batch_action_universal_state_init(
                    promise_index,
                    &state_init,
                    deposit,
                )
            }
        }
    }
}

/// Actions and refund configuration shared by receipt phases, never by joints.
#[derive(Default)]
struct PromiseActions {
    actions: Vec<PromiseAction>,
    #[cfg(feature = "deterministic-account-ids")]
    refund_to: Option<AccountId>,
}

impl PromiseActions {
    fn construct(&mut self, promise_index: PromiseIndex) {
        #[cfg(feature = "deterministic-account-ids")]
        if let Some(refund_to) = self.refund_to.take() {
            crate::env::promise_set_refund_to(promise_index, &refund_to);
        }
        for action in mem::take(&mut self.actions) {
            action.add(promise_index);
        }
    }
}

struct PromiseReceipt {
    account_id: AccountId,
    actions: PromiseActions,
    /// Promise index that is computed only once.
    promise_index: Option<PromiseIndex>,
}

impl PromiseReceipt {
    fn new(account_id: AccountId) -> Self {
        Self { account_id, actions: PromiseActions::default(), promise_index: None }
    }

    fn after(self, after: Rc<RefCell<Promise>>) -> PromiseLinked {
        PromiseLinked { receipt: self, after }
    }
}

struct PromiseLinked {
    receipt: PromiseReceipt,
    after: Rc<RefCell<Promise>>,
}

struct PromiseYielded {
    promise_index: PromiseIndex,
    actions: PromiseActions,
}

impl PromiseYielded {
    fn new(promise_index: PromiseIndex) -> Self {
        Self { promise_index, actions: PromiseActions::default() }
    }
}

struct PromiseJoint {
    promises: VecDeque<Promise>,
    /// Promise index that is computed only once.
    promise_index: Option<PromiseIndex>,
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

// All receipt phases use the same action builders and host-call implementation.
macro_rules! promise_actions {
    () => {
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

        /// Deploy a global smart contract using the provided contract code.
        /// Uses low-level [`crate::env::promise_batch_action_deploy_global_contract`]
        ///
        /// # Examples
        /// ```no_run
        /// use near_sdk::{Promise, NearToken};
        ///
        /// let code = vec![0u8; 100]; // Contract bytecode
        /// Promise::new("alice.near".parse().unwrap())
        ///     .create_account()
        ///     .transfer(NearToken::from_yoctonear(1000))
        ///     .deploy_global_contract(code);
        /// ```
        pub fn deploy_global_contract(self, code: impl Into<Vec<u8>>) -> Self {
            self.add_action(PromiseAction::DeployGlobalContract { code: code.into() })
        }

        /// Deploy a global smart contract, identifiable by the predecessor's account ID.
        /// Uses low-level [`crate::env::promise_batch_action_deploy_global_contract_by_account_id`]
        ///
        /// # Examples
        /// ```no_run
        /// use near_sdk::{Promise, NearToken};
        ///
        /// let code = vec![0u8; 100]; // Contract bytecode
        /// Promise::new("alice.near".parse().unwrap())
        ///     .create_account()
        ///     .transfer(NearToken::from_yoctonear(1000))
        ///     .deploy_global_contract_by_account_id(code);
        /// ```
        pub fn deploy_global_contract_by_account_id(self, code: impl Into<Vec<u8>>) -> Self {
            self.add_action(PromiseAction::DeployGlobalContractByAccountId { code: code.into() })
        }

        /// Use an existing global contract by code hash.
        /// Uses low-level [`crate::env::promise_batch_action_use_global_contract`]
        ///
        /// # Examples
        /// ```no_run
        /// use near_sdk::{Promise, NearToken};
        ///
        /// let code_hash = [0u8; 32]; // 32-byte hash (CryptoHash)
        /// Promise::new("alice.near".parse().unwrap())
        ///     .create_account()
        ///     .transfer(NearToken::from_yoctonear(1000))
        ///     .use_global_contract(code_hash);
        /// ```
        pub fn use_global_contract(self, code_hash: impl Into<CryptoHash>) -> Self {
            self.add_action(PromiseAction::UseGlobalContract { code_hash: code_hash.into() })
        }

        /// Use an existing global contract by referencing the account that deployed it.
        /// Uses low-level [`crate::env::promise_batch_action_use_global_contract_by_account_id`]
        ///
        /// # Examples
        /// ```no_run
        /// use near_sdk::{Promise, NearToken};
        ///
        /// Promise::new("alice.near".parse().unwrap())
        ///     .create_account()
        ///     .transfer(NearToken::from_yoctonear(1000))
        ///     .use_global_contract_by_account_id("deployer.near".parse().unwrap());
        /// ```
        pub fn use_global_contract_by_account_id(self, account_id: AccountId) -> Self {
            self.add_action(PromiseAction::UseGlobalContractByAccountId { account_id })
        }

        /// Creates a deterministic account with the given code, deposit, and data.
        #[cfg(feature = "deterministic-account-ids")]
        pub fn state_init(
            self,
            state_init: crate::state_init::StateInit,
            deposit: NearToken,
        ) -> Self {
            self.add_action(PromiseAction::DeterministicStateInit { state_init, deposit })
        }

        /// Adds an action that creates the `0u` universal account for `state_init` and funds it with
        /// `deposit` ([NEP-655]).
        ///
        /// The promise's receiver must be the account id these exact bytes derive to
        /// ([`RawStateInit::try_derive_account_id`](crate::universal_state_init::RawStateInit::try_derive_account_id)
        /// or [`env::universal_state_init_to_account_id`](crate::env::universal_state_init_to_account_id),
        /// both of which reject non-canonical bytes).
        /// The runtime validates the new receipt when the calling function call finishes, and a
        /// mismatch fails that call and rolls back its state changes.
        ///
        /// `state_init` is either typed or raw; both convert into the
        /// [`RawStateInit`](crate::universal_state_init::RawStateInit) bytes the action carries.
        /// A typed [`UniversalStateInit`](crate::universal_state_init::UniversalStateInit) (or
        /// [`UniversalStateInitV1`](crate::universal_state_init::UniversalStateInitV1)) is encoded
        /// once here, canonically, so its
        /// [`derive_account_id`](crate::universal_state_init::UniversalStateInit::derive_account_id)
        /// cannot fail. A `RawStateInit` goes in verbatim, which suits forwarding, including versions
        /// this SDK cannot decode. Forward bytes you were handed as they are: decoding and re-encoding
        /// them can change the account they derive to.
        ///
        /// Uses low-level [`crate::env::promise_batch_action_universal_state_init`].
        ///
        /// # Requirements
        ///
        /// Requires the host to support universal accounts (nearcore protocol version 88+, shipped in
        /// nearcore 2.14).
        ///
        /// # Examples
        ///
        /// A factory that validates locally supported state-init versions before scheduling an
        /// action, taking the original bytes as a base64 JSON argument. Invalid input is rejected
        /// before constructing a promise. To also accept versions this SDK predates, validate with
        /// [`env::universal_state_init_to_account_id`](crate::env::universal_state_init_to_account_id)
        /// instead, which asks the host.
        ///
        /// ```no_run
        /// use near_sdk::universal_state_init::RawStateInit;
        /// use near_sdk::{env, near, Promise};
        ///
        /// #[near(contract_state)]
        /// #[derive(Default)]
        /// pub struct Factory;
        ///
        /// #[near]
        /// impl Factory {
        ///     #[payable]
        ///     pub fn create(&mut self, state_init: RawStateInit) -> Promise {
        ///         let account_id = state_init.try_derive_account_id()
        ///             .unwrap_or_else(|_| env::panic_str("Invalid or non-canonical state init"));
        ///         Promise::new(account_id)
        ///             .universal_state_init(state_init, env::attached_deposit())
        ///     }
        /// }
        /// ```
        ///
        /// Building a typed state init in the contract, which needs no validation:
        ///
        /// ```no_run
        /// use near_sdk::universal_state_init::{UniversalStateInit, UniversalStateInitV1};
        /// use near_sdk::{env, GlobalContractId, NearToken, Promise};
        ///
        /// let state_init = UniversalStateInit::V1(
        ///     UniversalStateInitV1::default()
        ///         .with_code(GlobalContractId::AccountId("code.near".parse().unwrap()))
        ///         .with_access_key(env::signer_account_pk()),
        /// );
        /// Promise::new(state_init.derive_account_id())
        ///     .universal_state_init(state_init, NearToken::from_millinear(10));
        /// ```
        ///
        /// [NEP-655]: https://github.com/near/NEPs/pull/655
        pub fn universal_state_init(
            self,
            state_init: impl Into<crate::universal_state_init::RawStateInit>,
            deposit: NearToken,
        ) -> Self {
            self.add_action(PromiseAction::UniversalStateInit {
                state_init: state_init.into(),
                deposit,
            })
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

        /// Transfer tokens to a gas key on the account that this promise acts on.
        /// Uses low-level [`crate::env::promise_batch_action_transfer_to_gas_key`]
        ///
        /// # Requirements
        ///
        /// Requires the host to support gas-key host functions (nearcore protocol version 85+,
        /// shipped in nearcore 2.13).
        pub fn transfer_to_gas_key(self, public_key: PublicKey, amount: NearToken) -> Self {
            self.add_action(PromiseAction::TransferToGasKey { public_key, amount })
        }

        /// Add a full access gas key to the given account, reserving `num_nonces` parallel nonces.
        /// Uses low-level [`crate::env::promise_batch_action_add_gas_key_with_full_access`]
        ///
        /// # Requirements
        ///
        /// Requires the host to support gas-key host functions (nearcore protocol version 85+,
        /// shipped in nearcore 2.13).
        pub fn add_gas_key_full_access(self, public_key: PublicKey, num_nonces: u32) -> Self {
            self.add_action(PromiseAction::AddGasKeyFullAccess { public_key, num_nonces })
        }

        /// Add a gas key restricted to only calling a smart contract on some account using only a
        /// restricted set of methods, reserving `num_nonces` parallel nonces. Here `function_names`
        /// is a comma separated list of methods, e.g. `"method_a,method_b"`.
        /// Uses low-level [`crate::env::promise_batch_action_add_gas_key_with_function_call`]
        ///
        /// # Requirements
        ///
        /// Requires the host to support gas-key host functions (nearcore protocol version 85+,
        /// shipped in nearcore 2.13).
        pub fn add_gas_key_allowance_function_call(
            self,
            public_key: PublicKey,
            num_nonces: u32,
            allowance: Allowance,
            receiver_id: AccountId,
            function_names: String,
        ) -> Self {
            self.add_action(PromiseAction::AddGasKeyFunctionCall {
                public_key,
                num_nonces,
                allowance,
                receiver_id,
                function_names,
            })
        }
    };
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
///         contract_b::ext("bob_near".parse().unwrap()).b()
///     }
/// }
/// ```
///
/// * When they need to create a transaction with one or many actions, e.g. the following code
///   schedules a transaction that creates an account, transfers tokens, and assigns a public key:
///
/// ```no_run
/// # use near_sdk::{Promise, env, test_utils::VMContextBuilder, testing_env, Gas, NearToken};
/// # testing_env!(VMContextBuilder::new().signer_account_id("bob_near".parse().unwrap())
/// #               .account_balance(NearToken::from_yoctonear(1000)).prepaid_gas(Gas::from_gas(1_000_000)).build());
/// Promise::new("bob_near".parse().unwrap())
///   .create_account()
///   .transfer(NearToken::from_yoctonear(1000))
///   .add_full_access_key(env::signer_account_pk());
/// ```
///
/// More information about promises in [NEAR documentation](https://docs.near.org/smart-contracts/anatomy/crosscontract#promises)
#[must_use = "return or detach explicitly via `.detach()`"]
pub struct Promise {
    subtype: PromiseSubtype,
    // Serialization takes &self, so return marking still needs interior mutability.
    should_return: RefCell<bool>,
}

/// Promise serialization describes the returned value, so its Borsh schema remains unit.
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
    Fresh(PromiseReceipt),
    Linked(PromiseLinked),
    Yielded(PromiseYielded),
    Joint(PromiseJoint),
}

impl Promise {
    /// Create a promise that acts on the given account.
    /// Uses low-level [`crate::env::promise_batch_create`]
    pub fn new(account_id: AccountId) -> Self {
        Self::new_with_subtype(PromiseSubtype::Fresh(PromiseReceipt::new(account_id)))
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
        function_name: &str,
        arguments: impl AsRef<[u8]>,
        gas: Gas,
        weight: GasWeight,
    ) -> (Self, YieldId) {
        let (promise_index, yield_id) =
            crate::env::promise_yield_create_id(function_name, arguments, gas, weight);
        let promise =
            Self::new_with_subtype(PromiseSubtype::Yielded(PromiseYielded::new(promise_index)));
        (promise, yield_id)
    }

    /// Create a yielded promise with a caller-provided [`UserYieldId`] and an optional `deposit`.
    ///
    /// Like [`Promise::new_yield`], but the caller supplies the resumption identifier instead of
    /// the runtime generating one. The promise is later resumed with the same id via
    /// [`UserYieldId::resume`], so the creator and the resumer can derive the id independently
    /// (e.g. from a request id) without round-tripping a runtime-generated `data_id`.
    ///
    /// Returns `None` if a yield with the same `yield_id` is already pending for the current
    /// account, letting the caller detect and recover from duplicates instead of aborting.
    ///
    /// # Arguments
    /// * `function_name` - The callback function to invoke when the promise is resumed
    /// * `arguments` - Arguments to pass to the callback function
    /// * `deposit` - Amount of NEAR to attach to the callback (use `NearToken::from_yoctonear(0)` for none)
    /// * `gas` - Base gas to allocate for the callback
    /// * `weight` - Gas weight for distributing remaining gas
    /// * `yield_id` - The caller-provided 32-byte resumption identifier
    ///
    /// # Requirements
    /// Requires the host to support the yield-with-id functions (nearcore protocol version 85+,
    /// shipped in nearcore 2.13).
    ///
    /// # Important
    /// Like [`Promise::new_yield`], the resulting promise cannot be used as a continuation
    /// (`other.then(yielded)` panics); it must come first in a chain.
    ///
    /// Uses low-level [`crate::env::promise_yield_create_with_id`]
    pub fn new_yield_with_id(
        function_name: &str,
        arguments: impl AsRef<[u8]>,
        deposit: NearToken,
        gas: Gas,
        weight: GasWeight,
        yield_id: UserYieldId,
    ) -> Option<Self> {
        let promise_index = crate::env::promise_yield_create_with_id(
            function_name,
            arguments,
            deposit,
            gas,
            weight,
            yield_id.as_bytes(),
        )?;
        let promise =
            Self::new_with_subtype(PromiseSubtype::Yielded(PromiseYielded::new(promise_index)));
        Some(promise)
    }

    /// Set a different [`AccountId`] instead of current one to which refunds
    /// should go for all failed (e.g. [function_call](Promise::function_call_weight))
    /// or unused (e.g. [state_init](Promise::state_init)) deposits in
    /// the created receipt.
    #[cfg(feature = "deterministic-account-ids")]
    pub fn refund_to(mut self, account_id: impl Into<AccountId>) -> Self {
        match &mut self.subtype {
            PromiseSubtype::Fresh(receipt) => &mut receipt.actions,
            PromiseSubtype::Linked(linked) => &mut linked.receipt.actions,
            PromiseSubtype::Yielded(yielded) => &mut yielded.actions,
            PromiseSubtype::Joint(_) => {
                crate::env::panic_str("Cannot set refund account for a joint promise.")
            }
        }
        .refund_to = Some(account_id.into());
        self
    }

    fn add_action(mut self, action: PromiseAction) -> Self {
        let actions = match &mut self.subtype {
            PromiseSubtype::Fresh(receipt) => &mut receipt.actions,
            PromiseSubtype::Linked(linked) => &mut linked.receipt.actions,
            PromiseSubtype::Yielded(yielded) => &mut yielded.actions,
            PromiseSubtype::Joint(_) => {
                crate::env::panic_str("Cannot add action to a joint promise.")
            }
        };
        actions.actions.push(action);
        self
    }

    promise_actions!();

    /// Merge this promise with another promise, so that we can schedule execution of another
    /// smart contract right after all merged promises finish.
    ///
    /// Note, once the promises are merged it is not possible to add actions to them, e.g. the
    /// following code will panic during the execution of the smart contract:
    ///
    /// ```no_run
    /// # use near_sdk::Promise;
    /// let p1 = Promise::new("bob_near".parse().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse().unwrap()).create_account();
    /// let p3 = p1.and(p2);
    /// // p3.create_account();
    /// ```
    /// Uses low-level [`crate::env::promise_and`]
    pub fn and(self, other: Promise) -> Promise {
        let (joint, should_return) = self.and_impl(other);
        Promise { subtype: PromiseSubtype::Joint(joint), should_return }
    }

    fn into_parts(mut self) -> (PromiseSubtype, RefCell<bool>) {
        let subtype = mem::replace(
            &mut self.subtype,
            PromiseSubtype::Joint(PromiseJoint { promises: VecDeque::new(), promise_index: None }),
        );
        (subtype, mem::take(&mut self.should_return))
    }

    fn and_impl(self, other: Promise) -> (PromiseJoint, RefCell<bool>) {
        // Preserve the joint's return flag and the existing left-to-right merge order.
        let (left, left_return) = self.into_parts();
        let (right, right_return) = other.into_parts();
        match (left, right) {
            (PromiseSubtype::Joint(mut left), PromiseSubtype::Joint(mut right)) => {
                left.promises.append(&mut right.promises);
                (left, left_return)
            }
            (PromiseSubtype::Joint(mut left), right) => {
                left.promises.push_back(Promise { subtype: right, should_return: right_return });
                (left, left_return)
            }
            (left, PromiseSubtype::Joint(mut right)) => {
                right.promises.push_front(Promise { subtype: left, should_return: left_return });
                (right, right_return)
            }
            (left, right) => (
                PromiseJoint {
                    promises: [
                        Promise { subtype: left, should_return: left_return },
                        Promise { subtype: right, should_return: right_return },
                    ]
                    .into(),
                    promise_index: None,
                },
                RefCell::new(false),
            ),
        }
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
    /// Uses low-level [`crate::env::promise_batch_then`]
    pub fn then(self, other: Promise) -> Promise {
        Promise::then_impl(Rc::new(RefCell::new(self)), other)
    }

    /// Shared, private implementation between `then` and `then_concurrent`.
    ///
    /// This takes self as a reference counted object promise, to allow multiple
    /// dependencies pointing to the same promise without duplicating that
    /// promise.
    fn then_impl(this: Rc<RefCell<Self>>, other: Promise) -> Promise {
        match &other.subtype {
            PromiseSubtype::Fresh(_) => {}
            PromiseSubtype::Linked(_) => crate::env::panic_str(
                "Cannot callback promise which is already scheduled after another",
            ),
            PromiseSubtype::Yielded(_) => crate::env::panic_str("Cannot callback yielded promise."),
            PromiseSubtype::Joint(_) => crate::env::panic_str("Cannot callback joint promise."),
        }
        let (subtype, should_return) = other.into_parts();
        let PromiseSubtype::Fresh(receipt) = subtype else { unreachable!() };
        Promise { subtype: PromiseSubtype::Linked(receipt.after(this)), should_return }
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
    /// # use near_sdk::Promise;
    /// let p1 = Promise::new("bob_near".parse().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse().unwrap()).create_account();
    /// let p3 = Promise::new("dave_near".parse().unwrap()).create_account();
    /// let p4 = Promise::new("eva_near".parse().unwrap()).create_account();
    /// p1.then(p2).then_concurrent([p3, p4]);
    /// ```
    ///
    /// The returned [`ConcurrentPromises`] allows chaining more promises.
    ///
    /// In the following code, `bob_near` is created first, next `carol_near`
    /// and `dave_near` are created concurrently, and finally `eva_near` is
    /// created after all others have been created.
    ///
    /// ```no_run
    /// # use near_sdk::Promise;
    /// let p1 = Promise::new("bob_near".parse().unwrap()).create_account();
    /// let p2 = Promise::new("carol_near".parse().unwrap()).create_account();
    /// let p3 = Promise::new("dave_near".parse().unwrap()).create_account();
    /// let p4 = Promise::new("eva_near".parse().unwrap()).create_account();
    /// p1.then_concurrent([p2, p3]).join().then(p4);
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
    /// # use near_sdk::{ext_contract, Gas, near, Promise};
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
    ///        contract_b::ext("bob_near".parse().unwrap()).b().as_return();
    ///     }
    ///
    ///     pub fn a2(&self) -> Promise {
    ///        contract_b::ext("bob_near".parse().unwrap()).b()
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
            PromiseSubtype::Fresh(receipt) => {
                let index = *receipt
                    .promise_index
                    .get_or_insert_with(|| crate::env::promise_batch_create(&receipt.account_id));
                receipt.actions.construct(index);
                index
            }
            PromiseSubtype::Linked(linked) => {
                let receipt = &mut linked.receipt;
                let index = *receipt.promise_index.get_or_insert_with(|| {
                    if let Some(after) = linked.after.borrow_mut().construct_recursively() {
                        crate::env::promise_batch_then(after, &receipt.account_id)
                    } else {
                        crate::env::promise_batch_create(&receipt.account_id)
                    }
                });
                receipt.actions.construct(index);
                index
            }
            PromiseSubtype::Yielded(yielded) => {
                yielded.actions.construct(yielded.promise_index);
                yielded.promise_index
            }
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

    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        // Since promises are untyped, for now we represent Promise results with the schema
        // `true` which matches everything (i.e. always passes validation)
        schemars::schema::Schema::Bool(true)
    }
}

// The Option is only a move-out slot: consuming a typed handle leaves it empty so
// Drop cannot schedule it again. It does not represent a promise phase.
struct PromiseHandle<S: Into<PromiseSubtype>> {
    state: Option<S>,
    should_return: RefCell<bool>,
}

impl<S: Into<PromiseSubtype>> PromiseHandle<S> {
    fn new(state: S) -> Self {
        Self { state: Some(state), should_return: RefCell::new(false) }
    }

    fn into_parts(mut self) -> (S, RefCell<bool>) {
        (
            self.state.take().expect("promise handle already consumed"),
            mem::take(&mut self.should_return),
        )
    }

    fn into_promise(self) -> Promise {
        let (state, should_return) = self.into_parts();
        Promise { subtype: state.into(), should_return }
    }
}

impl<S: Into<PromiseSubtype>> Drop for PromiseHandle<S> {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            drop(Promise {
                subtype: state.into(),
                should_return: mem::take(&mut self.should_return),
            });
        }
    }
}

// Concrete phase data is shared by the typed handles and the erased enum. Only
// conversion into Promise is exposed; erasure cannot be reversed by callers.
macro_rules! typed_promise {
    ($(#[$attr:meta])* $name:ident, $state:ty, $variant:ident) => {
        $(#[$attr])*
        #[must_use = "return or detach explicitly via `.detach()`"]
        pub struct $name(PromiseHandle<$state>);

        impl From<$state> for PromiseSubtype {
            fn from(state: $state) -> Self {
                Self::$variant(state)
            }
        }

        impl From<$name> for Promise {
            fn from(promise: $name) -> Self {
                promise.0.into_promise()
            }
        }

        impl<T> From<$name> for PromiseOrValue<T> {
            fn from(promise: $name) -> Self {
                Self::Promise(promise.into())
            }
        }

        impl $name {
            /// Merge with any promise phase, consuming both promises.
            pub fn and(self, other: impl Into<Promise>) -> JointPromise {
                let (joint, should_return) = Promise::from(self).and_impl(other.into());
                JointPromise(PromiseHandle { state: Some(joint), should_return })
            }

            /// Link a fresh receipt after this promise, consuming both phases.
            /// A linked, yielded, or joint promise cannot be a continuation.
            pub fn then(self, continuation: FreshPromise) -> LinkedPromise {
                continuation.after(Rc::new(RefCell::new(self.into())))
            }

            /// Link fresh receipts to a single shared predecessor.
            ///
            /// The returned collection erases the linked phases, preserving
            /// [`ConcurrentPromises::join`] and its behavior for any list length.
            pub fn then_concurrent(
                self,
                promises: impl IntoIterator<Item = FreshPromise>,
            ) -> ConcurrentPromises {
                let this = Rc::new(RefCell::new(self.into()));
                ConcurrentPromises {
                    promises: promises.into_iter()
                        .map(|promise| promise.after(Rc::clone(&this)).into())
                        .collect(),
                }
            }

            /// Mark this promise as the return value while preserving its phase.
            #[allow(clippy::wrong_self_convention)]
            pub fn as_return(self) -> Self {
                *self.0.should_return.borrow_mut() = true;
                self
            }

            /// Explicitly detach this promise, scheduling it on drop.
            pub fn detach(self) {}
        }
    };
}

typed_promise!(
    /// A fresh receipt, eligible to become a continuation exactly once.
    ///
    /// Use [`FreshPromise::new`] to opt into phase checking. Convert into
    /// [`Promise`] when an erased return type is needed; there is intentionally
    /// no conversion from `Promise` back to a specific phase.
    ///
    /// ```no_run
    /// use near_sdk::{FreshPromise, NearToken, Promise};
    /// let promise: Promise = FreshPromise::new("alice.near".parse().unwrap())
    ///     .create_account()
    ///     .then(FreshPromise::new("bob.near".parse().unwrap()))
    ///     .transfer(NearToken::from_yoctonear(1))
    ///     .into();
    /// ```
    FreshPromise, PromiseReceipt, Fresh
);

typed_promise!(
    /// A receipt linked to its predecessor. It accepts actions but cannot be
    /// used as another promise's continuation.
    ///
    /// ```compile_fail
    /// use near_sdk::FreshPromise;
    /// let callback = FreshPromise::new("alice.near".parse().unwrap())
    ///     .then(FreshPromise::new("bob.near".parse().unwrap()));
    /// FreshPromise::new("carol.near".parse().unwrap()).then(callback);
    /// ```
    ///
    /// Concurrent continuations must also be fresh:
    ///
    /// ```compile_fail
    /// use near_sdk::FreshPromise;
    /// let callback = FreshPromise::new("alice.near".parse().unwrap())
    ///     .then(FreshPromise::new("bob.near".parse().unwrap()));
    /// FreshPromise::new("carol.near".parse().unwrap()).then_concurrent([callback]);
    /// ```
    LinkedPromise, PromiseLinked, Linked
);

typed_promise!(
    /// A yielded receipt with an assigned host index and no predecessor slot.
    /// It accepts actions and can precede a fresh receipt, but cannot be a continuation.
    ///
    /// ```compile_fail
    /// use near_sdk::{FreshPromise, YieldedPromise, Gas, GasWeight};
    /// let (yielded, _) = YieldedPromise::new("resume", [], Gas::from_tgas(5), GasWeight(1));
    /// FreshPromise::new("alice.near".parse().unwrap()).then(yielded);
    /// ```
    YieldedPromise, PromiseYielded, Yielded
);

typed_promise!(
    /// A joint promise. Only joining, chaining, returning, and detaching are
    /// available; a joint owns no actions or receipt configuration.
    ///
    /// ```compile_fail
    /// use near_sdk::FreshPromise;
    /// let joint = FreshPromise::new("alice.near".parse().unwrap())
    ///     .and(FreshPromise::new("bob.near".parse().unwrap()));
    /// joint.create_account();
    /// ```
    ///
    /// A joint cannot be used as a continuation:
    ///
    /// ```compile_fail
    /// use near_sdk::FreshPromise;
    /// let joint = FreshPromise::new("alice.near".parse().unwrap())
    ///     .and(FreshPromise::new("bob.near".parse().unwrap()));
    /// FreshPromise::new("carol.near".parse().unwrap()).then(joint);
    /// ```
    JointPromise, PromiseJoint, Joint
);

impl FreshPromise {
    /// Create a fresh receipt acting on the given account.
    pub fn new(account_id: AccountId) -> Self {
        Self(PromiseHandle::new(PromiseReceipt::new(account_id)))
    }

    fn after(self, after: Rc<RefCell<Promise>>) -> LinkedPromise {
        let (receipt, should_return) = self.0.into_parts();
        LinkedPromise(PromiseHandle { state: Some(receipt.after(after)), should_return })
    }
}

impl YieldedPromise {
    /// Create a yielded receipt and its runtime-generated resumption identifier.
    /// See [`Promise::new_yield`].
    pub fn new(
        function_name: &str,
        arguments: impl AsRef<[u8]>,
        gas: Gas,
        weight: GasWeight,
    ) -> (Self, YieldId) {
        let (promise_index, yield_id) =
            crate::env::promise_yield_create_id(function_name, arguments, gas, weight);
        (Self(PromiseHandle::new(PromiseYielded::new(promise_index))), yield_id)
    }

    /// Create a yielded receipt using a caller-provided resumption identifier.
    /// See [`Promise::new_yield_with_id`].
    pub fn new_with_id(
        function_name: &str,
        arguments: impl AsRef<[u8]>,
        deposit: NearToken,
        gas: Gas,
        weight: GasWeight,
        yield_id: UserYieldId,
    ) -> Option<Self> {
        let promise_index = crate::env::promise_yield_create_with_id(
            function_name,
            arguments,
            deposit,
            gas,
            weight,
            yield_id.as_bytes(),
        )?;
        Some(Self(PromiseHandle::new(PromiseYielded::new(promise_index))))
    }
}

macro_rules! typed_receipt_actions {
    ($name:ident, $($field:ident).+) => {
        impl $name {
            fn actions(&mut self) -> &mut PromiseActions {
                &mut self.0.state.as_mut().expect("promise handle already consumed").$($field).+
            }

            fn add_action(mut self, action: PromiseAction) -> Self {
                self.actions().actions.push(action);
                self
            }

            /// Set the refund account for this receipt. See [`Promise::refund_to`].
            #[cfg(feature = "deterministic-account-ids")]
            pub fn refund_to(mut self, account_id: impl Into<AccountId>) -> Self {
                self.actions().refund_to = Some(account_id.into());
                self
            }

            promise_actions!();
        }
    };
}

typed_receipt_actions!(FreshPromise, actions);
typed_receipt_actions!(LinkedPromise, receipt.actions);
typed_receipt_actions!(YieldedPromise, actions);

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
        if crate::env::promise_yield_resume(&self.0, data) { Ok(()) } else { Err(ResumeError) }
    }
}

/// A caller-provided identifier for a yielded promise created with [`Promise::new_yield_with_id`].
///
/// Unlike [`YieldId`] — which wraps the runtime-generated `data_id` returned by
/// [`Promise::new_yield`] — a `UserYieldId` is chosen by the contract itself (any 32 bytes, e.g.
/// derived from a request id). Because both the creator and the resumer can compute it
/// independently, the resumer doesn't need the creator to hand back a runtime-generated token.
///
/// # Example
/// ```ignore
/// use near_sdk::{Promise, NearToken, Gas, GasWeight, UserYieldId};
///
/// let yield_id = UserYieldId::new([7u8; 32]);
/// let promise = Promise::new_yield_with_id(
///     "on_resume",
///     vec![],
///     NearToken::from_yoctonear(0),
///     Gas::from_tgas(5),
///     GasWeight(1),
///     yield_id,
/// )
/// .expect("a yield with this id is already pending");
///
/// // Later, from another transaction, resume with the same id:
/// yield_id.resume(b"result data").unwrap();
/// ```
#[near(inside_nearsdk, serializers = [json, borsh])]
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct UserYieldId(
    #[serde_as(as = "::serde_with::base64::Base64")]
    #[cfg_attr(feature = "abi", schemars(with = "String"))]
    pub CryptoHash,
);

impl UserYieldId {
    /// Construct a `UserYieldId` from 32 raw bytes.
    pub const fn new(bytes: CryptoHash) -> Self {
        Self(bytes)
    }

    /// The 32-byte identifier.
    pub const fn as_bytes(&self) -> &CryptoHash {
        &self.0
    }

    /// Resume the yielded promise created with this id, passing `data` to the callback as a
    /// promise result.
    ///
    /// Returns `Ok(())` if a pending yield with this id was found and resume was submitted, or
    /// `Err(ResumeError)` if none exists (already resumed, timed out, or never created).
    ///
    /// Uses low-level [`crate::env::promise_yield_resume_with_yield_id`]
    pub fn resume(&self, data: impl AsRef<[u8]>) -> Result<(), ResumeError> {
        if crate::env::promise_yield_resume_with_yield_id(&self.0, data) {
            Ok(())
        } else {
            Err(ResumeError)
        }
    }
}

impl From<CryptoHash> for UserYieldId {
    fn from(bytes: CryptoHash) -> Self {
        Self(bytes)
    }
}

/// When the method can return either a promise or a value, it can be called with `PromiseOrValue::Promise`
/// or `PromiseOrValue::Value` to specify which one should be returned.
/// # Example
/// ```no_run
/// # use near_sdk::{ext_contract, near, Gas, PromiseOrValue};
/// #[ext_contract]
/// pub trait ContractA {
///     fn a(&mut self);
/// }
///
/// let value = Some(true);
/// let val: PromiseOrValue<bool> = if let Some(value) = value {
///     PromiseOrValue::Value(value)
/// } else {
///     contract_a::ext("bob_near".parse().unwrap()).a().into()
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

    fn json_schema(r#gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        T::json_schema(r#gen)
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
        AccountId, Allowance, NearToken, Promise, PublicKey, test_utils::VMContextBuilder,
        testing_env,
    };

    use super::{FreshPromise, LinkedPromise, YieldedPromise};
    use crate::{Gas, GasWeight, PromiseOrValue, UserYieldId};

    fn receipts_from(build: impl FnOnce()) -> Vec<crate::mock::Receipt> {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());
        build();
        get_created_receipts()
    }

    #[test]
    fn typed_actions_match_erased_actions() {
        macro_rules! actions {
            ($promise:expr) => {
                $promise
                    .create_account()
                    .transfer(NearToken::from_yoctonear(7))
                    .deploy_contract(vec![1, 2, 3])
                    .function_call(
                        "call",
                        vec![4, 5],
                        NearToken::from_yoctonear(2),
                        Gas::from_tgas(1),
                    )
                    .function_call_weight(
                        "weighted",
                        vec![6],
                        NearToken::from_yoctonear(3),
                        Gas::from_tgas(2),
                        GasWeight(2),
                    )
                    .stake(NearToken::from_yoctonear(1), pk())
                    .add_full_access_key_with_nonce(pk(), 42)
                    .add_access_key_allowance_with_nonce(
                        pk(),
                        Allowance::Unlimited,
                        bob(),
                        "a,b",
                        17,
                    )
                    .delete_key(pk())
                    .delete_account(bob())
            };
        }
        let erased = receipts_from(|| actions!(Promise::new(alice())).detach());
        let typed = receipts_from(|| actions!(FreshPromise::new(alice())).detach());
        assert_eq!(typed, erased);
        assert_eq!(typed[0].actions.len(), 10);

        let erased = receipts_from(|| {
            actions!(Promise::new(bob()).then(Promise::new(alice()))).detach();
        });
        let typed = receipts_from(|| {
            actions!(FreshPromise::new(bob()).then(FreshPromise::new(alice()))).detach();
        });
        assert_eq!(typed, erased);
    }

    #[test]
    fn typed_joint_merges_match_erased_merges() {
        macro_rules! graph {
            ($fresh:ident, $shape:expr) => {{
                let a = $fresh::new(alice()).create_account();
                let b = $fresh::new(bob()).transfer(NearToken::from_yoctonear(2));
                let c = $fresh::new("carol.near".parse().unwrap()).create_account();
                let d = $fresh::new("dave.near".parse().unwrap()).create_account();
                // Cover both joints, either side joint, and neither side joint.
                let joint = match $shape {
                    0 => a.and(b).and(c.and(d)),
                    1 => a.and(b.and(c)).and(d),
                    _ => a.and(b).and(c).and(d),
                };
                joint
                    .then($fresh::new("callback.near".parse().unwrap()))
                    .transfer(NearToken::from_yoctonear(3))
                    .detach();
            }};
        }
        for shape in 0..3 {
            let erased = receipts_from(|| graph!(Promise, shape));
            let typed = receipts_from(|| graph!(FreshPromise, shape));
            assert_eq!(typed, erased, "merge shape {shape}");
            assert_eq!(typed.len(), 5);
        }
    }

    #[test]
    fn typed_concurrent_promises_share_predecessor() {
        macro_rules! graph {
            ($fresh:ident, $split:expr) => {{
                let mut promises = $fresh::new(alice())
                    .create_account()
                    .then($fresh::new(bob()).create_account())
                    .then_concurrent([
                        $fresh::new("carol.near".parse().unwrap()).create_account(),
                        $fresh::new("dave.near".parse().unwrap()).create_account(),
                    ]);
                if $split {
                    promises
                        .split_off(1)
                        .join()
                        .then(Promise::new("callback.near".parse().unwrap()))
                        .detach();
                    promises.detach();
                } else {
                    promises.join().then(Promise::new("callback.near".parse().unwrap())).detach();
                }
            }};
        }
        for split in [false, true] {
            let erased = receipts_from(|| graph!(Promise, split));
            let typed = receipts_from(|| graph!(FreshPromise, split));
            assert_eq!(typed, erased);
            assert_eq!(typed.len(), 5, "shared predecessor must be created once");
        }
    }

    #[test]
    fn typed_drop_and_erasure_schedule_once() {
        let erased = receipts_from(|| {
            Promise::new(alice()).then(Promise::new(bob()).create_account()).detach();
        });
        let typed = receipts_from(|| {
            let linked: LinkedPromise =
                FreshPromise::new(alice()).then(FreshPromise::new(bob()).create_account());
            let erased: Promise = linked.into();
            assert!(get_created_receipts().is_empty(), "erasure must not schedule receipts");
            drop(erased);
        });
        assert_eq!(typed, erased);
        let typed = receipts_from(|| {
            let _linked =
                FreshPromise::new(alice()).then(FreshPromise::new(bob()).create_account());
        });
        assert_eq!(typed, erased);
    }

    #[test]
    fn typed_return_and_unit_serialization_preserve_erased_behavior() {
        let erased = receipts_from(|| {
            Promise::new(alice()).then(Promise::new(bob())).as_return().detach();
        });
        let typed = receipts_from(|| {
            let linked = FreshPromise::new(alice()).then(FreshPromise::new(bob())).as_return();
            let promise: Promise = linked.into();
            assert!(*promise.should_return.borrow());
            promise.detach();
        });
        assert_eq!(typed, erased);

        // Serialization takes &self, so return marking still needs RefCell<bool>.
        for borsh in [false, true] {
            let typed = receipts_from(|| {
                let promise: Promise = FreshPromise::new(alice()).into();
                assert!(!*promise.should_return.borrow());
                if borsh {
                    assert!(borsh::to_vec(&promise).unwrap().is_empty());
                } else {
                    assert_eq!(serde_json::to_string(&promise).unwrap(), "null");
                }
                assert!(*promise.should_return.borrow());
                promise.detach();
            });
            assert_eq!(typed.len(), 1);
        }
        let typed = receipts_from(|| {
            let promise: PromiseOrValue<()> = FreshPromise::new(alice()).into();
            assert_eq!(serde_json::to_string(&promise).unwrap(), "null");
            promise.detach();
        });
        assert_eq!(typed.len(), 1);
    }

    #[test]
    fn typed_yield_matches_erased_yield() {
        let erased = receipts_from(|| {
            let (promise, _) =
                Promise::new_yield("resume", [1, 2], Gas::from_tgas(5), GasWeight(1));
            promise.detach();
        });
        let typed = receipts_from(|| {
            let (promise, _) =
                YieldedPromise::new("resume", [1, 2], Gas::from_tgas(5), GasWeight(1));
            let promise: Promise = promise.as_return().into();
            assert!(*promise.should_return.borrow());
            promise.detach();
        });
        assert_eq!(typed, erased);

        let erased = receipts_from(|| {
            Promise::new_yield_with_id(
                "resume",
                [3],
                NearToken::from_yoctonear(0),
                Gas::from_tgas(5),
                GasWeight(1),
                UserYieldId::new([4; 32]),
            )
            .unwrap()
            .detach();
        });
        let typed = receipts_from(|| {
            let promise = YieldedPromise::new_with_id(
                "resume",
                [3],
                NearToken::from_yoctonear(0),
                Gas::from_tgas(5),
                GasWeight(1),
                UserYieldId::new([4; 32]),
            )
            .unwrap();
            assert!(
                YieldedPromise::new_with_id(
                    "resume",
                    [3],
                    NearToken::from_yoctonear(0),
                    Gas::from_tgas(5),
                    GasWeight(1),
                    UserYieldId::new([4; 32])
                )
                .is_none()
            );
            promise.detach();
        });
        assert_eq!(typed, erased);
    }

    #[cfg(feature = "deterministic-account-ids")]
    #[test]
    fn typed_refunds_match_erased_refunds() {
        let erased = receipts_from(|| {
            Promise::new(alice())
                .refund_to(bob())
                .then(Promise::new(bob()).create_account())
                .refund_to(alice())
                .detach();
        });
        let typed = receipts_from(|| {
            FreshPromise::new(alice())
                .refund_to(bob())
                .then(FreshPromise::new(bob()).create_account())
                .refund_to(alice())
                .detach();
        });
        assert_eq!(typed, erased);
    }

    #[test]
    #[should_panic(expected = "Cannot add action to a joint promise.")]
    fn erased_joint_still_rejects_actions() {
        testing_env!(VMContextBuilder::new().build());
        Promise::new(alice()).and(Promise::new(bob())).create_account().detach();
    }

    #[test]
    #[should_panic(expected = "Cannot callback joint promise.")]
    fn erased_joint_still_rejects_being_a_continuation() {
        testing_env!(VMContextBuilder::new().build());
        Promise::new(alice()).then(Promise::new(bob()).and(Promise::new(alice()))).detach();
    }

    #[test]
    #[should_panic(expected = "Cannot callback promise which is already scheduled after another")]
    fn erased_linked_still_rejects_being_a_continuation() {
        testing_env!(VMContextBuilder::new().build());
        Promise::new(alice()).then(Promise::new(bob()).then(Promise::new(alice()))).detach();
    }

    #[test]
    fn promise_indices_and_actions_are_constructed_once() {
        for phase in 0..3 {
            testing_env!(VMContextBuilder::new().build());
            let mut promise: Promise = match phase {
                0 => FreshPromise::new(alice()).create_account().into(),
                1 => FreshPromise::new(alice())
                    .create_account()
                    .then(FreshPromise::new(bob()).transfer(NearToken::from_yoctonear(7)))
                    .into(),
                _ => FreshPromise::new(alice())
                    .create_account()
                    .and(FreshPromise::new(bob()).transfer(NearToken::from_yoctonear(7)))
                    .into(),
            };
            let index = promise.construct_recursively();
            let receipts = get_created_receipts();
            assert_eq!(receipts.len(), if phase == 0 { 1 } else { 2 });
            assert_eq!(promise.construct_recursively(), index);
            assert_eq!(get_created_receipts(), receipts);
            drop(promise);
            assert_eq!(get_created_receipts(), receipts, "drop must reuse the cached index");
        }
    }

    #[test]
    fn yielded_actions_share_construction_and_cached_index() {
        // created_receipts omits yields. Compare host gas as well as the assigned
        // index and action drain, so two empty receipt lists cannot pass this test.
        fn construct(mut promise: Promise) -> (crate::PromiseIndex, Gas) {
            let before = crate::env::used_gas();
            let index = promise.construct_recursively().unwrap();
            let after = crate::env::used_gas();
            let super::PromiseSubtype::Yielded(yielded) = &promise.subtype else {
                panic!("expected a yielded promise");
            };
            assert!(yielded.actions.actions.is_empty());
            assert_eq!(yielded.promise_index, index);
            assert_eq!(promise.construct_recursively(), Some(index));
            drop(promise);
            (index, Gas::from_gas(after.as_gas() - before.as_gas()))
        }

        testing_env!(VMContextBuilder::new().build());
        let (erased, _) = Promise::new_yield("resume", [1, 2], Gas::from_tgas(5), GasWeight(1));
        let erased = construct(erased.transfer(NearToken::from_yoctonear(7)));

        testing_env!(VMContextBuilder::new().build());
        let (typed, _) = YieldedPromise::new("resume", [1, 2], Gas::from_tgas(5), GasWeight(1));
        let typed = construct(typed.transfer(NearToken::from_yoctonear(7)).into());
        assert_eq!(typed, erased);

        testing_env!(VMContextBuilder::new().build());
        let (empty, _) = Promise::new_yield("resume", [1, 2], Gas::from_tgas(5), GasWeight(1));
        let empty = construct(empty);
        assert!(typed.1 > empty.1, "the transfer must reach the host");
    }

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
    fn test_transfer_to_gas_key() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let amount = NearToken::from_yoctonear(1000);

        {
            Promise::new(alice())
                .create_account()
                .transfer_to_gas_key(public_key.clone(), amount)
                .detach();
        }

        let public_key = near_crypto::PublicKey::try_from(public_key).unwrap();
        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::TransferToGasKey { public_key: p, deposit, receipt_index: _ }
                if p == public_key && deposit == amount
            )
        });
        assert!(has_action);
    }

    #[test]
    fn test_add_gas_key_full_access() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();

        {
            Promise::new(alice())
                .create_account()
                .add_gas_key_full_access(public_key.clone(), 3)
                .detach();
        }

        let public_key = near_crypto::PublicKey::try_from(public_key).unwrap();
        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::AddGasKeyWithFullAccess { public_key: p, num_nonces: 3, receipt_index: _ }
                if p == public_key
            )
        });
        assert!(has_action);
    }

    #[test]
    fn test_add_gas_key_allowance_function_call() {
        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let public_key: PublicKey = pk();
        let allowance = 100;
        let receiver_id = bob();
        let function_names = "method_a,method_b".to_string();

        {
            Promise::new(alice())
                .create_account()
                .add_gas_key_allowance_function_call(
                    public_key.clone(),
                    5,
                    Allowance::Limited(allowance.try_into().unwrap()),
                    receiver_id.clone(),
                    function_names.clone(),
                )
                .detach();
        }

        let public_key = near_crypto::PublicKey::try_from(public_key).unwrap();
        let has_action = get_actions().any(|el| {
            matches!(
                el,
                MockAction::AddGasKeyWithFunctionCall {
                    public_key: p,
                    num_nonces: 5,
                    allowance: a,
                    receiver_id: r,
                    method_names,
                    receipt_index: _,
                }
                if p == public_key
                    && a == Some(NearToken::from_yoctonear(allowance))
                    && r == receiver_id
                    && method_names == function_names
                        .split(',')
                        .map(|s| s.as_bytes().to_vec())
                        .collect::<Vec<_>>()
            )
        });
        assert!(has_action);
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
    fn test_allowance_debug() {
        let unlimited = Allowance::Unlimited;
        assert_eq!(format!("{:?}", unlimited), "Unlimited");

        let limited = Allowance::Limited(100.try_into().unwrap());
        assert_eq!(format!("{:?}", limited), "Limited(100)");
    }

    #[test]
    fn test_allowance_eq() {
        assert_eq!(Allowance::Unlimited, Allowance::Unlimited);
        assert_eq!(
            Allowance::Limited(100.try_into().unwrap()),
            Allowance::Limited(100.try_into().unwrap())
        );
        assert_ne!(Allowance::Unlimited, Allowance::Limited(100.try_into().unwrap()));
        assert_ne!(
            Allowance::Limited(100.try_into().unwrap()),
            Allowance::Limited(200.try_into().unwrap())
        );
    }

    #[test]
    fn test_allowance_copy() {
        let a = Allowance::Unlimited;
        let b = a; // Copy
        assert_eq!(a, b);

        let c = Allowance::Limited(500.try_into().unwrap());
        let d = c; // Copy
        assert_eq!(c, d);
    }

    #[test]
    fn test_allowance_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Allowance::Unlimited);
        set.insert(Allowance::Limited(100.try_into().unwrap()));
        set.insert(Allowance::Unlimited); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_allowance_limited_zero_returns_none() {
        assert!(Allowance::limited(NearToken::from_yoctonear(0)).is_none());
    }

    #[test]
    fn test_allowance_limited_nonzero() {
        let allowance = Allowance::limited(NearToken::from_yoctonear(100));
        assert_eq!(allowance, Some(Allowance::Limited(100.try_into().unwrap())));
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

    #[test]
    fn test_new_yield_with_id_returns_promise() {
        use crate::{Gas, GasWeight, UserYieldId};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let yield_id = UserYieldId::new([7u8; 32]);
        let promise = Promise::new_yield_with_id(
            "on_callback",
            b"args",
            NearToken::from_yoctonear(0),
            Gas::from_tgas(10),
            GasWeight(1),
            yield_id,
        );

        assert!(promise.is_some(), "new_yield_with_id should return Some for a fresh yield id");
    }

    #[test]
    fn test_new_yield_with_id_duplicate_returns_none() {
        use crate::{Gas, GasWeight, UserYieldId};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let yield_id = UserYieldId::new([3u8; 32]);
        let make = || {
            Promise::new_yield_with_id(
                "on_callback",
                vec![],
                NearToken::from_yoctonear(0),
                Gas::from_tgas(5),
                GasWeight(1),
                yield_id,
            )
        };

        let _first = make().expect("first create with a fresh id should succeed");
        assert!(
            make().is_none(),
            "second create with the same yield id should return None (duplicate detected)"
        );
    }

    #[test]
    fn test_user_yield_id_resume() {
        use crate::{Gas, GasWeight, UserYieldId};

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        let yield_id = UserYieldId::new([9u8; 32]);
        let _promise = Promise::new_yield_with_id(
            "on_resume",
            vec![],
            NearToken::from_yoctonear(0),
            Gas::from_tgas(5),
            GasWeight(1),
            yield_id,
        )
        .expect("create with a fresh id should succeed");

        assert!(
            yield_id.resume(b"payload").is_ok(),
            "resuming a pending yield by its id should succeed"
        );
    }

    #[test]
    fn test_user_yield_id_resume_unknown_returns_err() {
        use crate::UserYieldId;

        testing_env!(VMContextBuilder::new().signer_account_id(alice()).build());

        // No yield was ever created for this id.
        let unknown = UserYieldId::new([1u8; 32]);
        assert!(
            unknown.resume(b"payload").is_err(),
            "resuming an unknown yield id should return Err(ResumeError)"
        );
    }
}
