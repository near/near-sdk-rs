use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use near_crypto::{InMemorySigner, KeyType, PublicKey, Signer};

use near_sdk::AccountId;
use near_sdk::PendingContractTx;

use crate::runtime::init_runtime;
pub use crate::to_yocto;
use crate::{
    account::{AccessKey, Account},
    hash::CryptoHash,
    outcome_into_result,
    runtime::{GenesisConfig, RuntimeStandalone},
    transaction::Transaction,
    types::{Balance, Gas},
    ExecutionResult, ViewResult,
};

pub const DEFAULT_GAS: u64 = 300_000_000_000_000;
pub const STORAGE_AMOUNT: u128 = 50_000_000_000_000_000_000_000_000;

type Runtime = Rc<RefCell<RuntimeStandalone>>;

/// A transaction to be signed by the user which created it. Multiple actions can be chained together
/// and then signed and sumited to be executed.
///
/// # Example:
///
/// ```
/// use near_sdk_sim::{to_yocto, account::AccessKey};
/// use near_crypto::{InMemorySigner, KeyType, Signer};
/// let master_account = near_sdk_sim::init_simulator(None);
/// let account_id = "alice";
/// let transaction = master_account.create_transaction(account_id.parse().unwrap());
/// // Creates a signer which contains a public key.
/// let signer = InMemorySigner::from_seed(account_id, KeyType::ED25519, account_id);
/// let res = transaction.create_account()
///                      .add_key(signer.public_key(), AccessKey::full_access())
///                      .transfer(to_yocto("10"))
///                      .submit();
/// ```
///
/// This creates an account for `alice`, and a new key pair for the account, adding the
/// public key to the account, and finally transfering `10` NEAR to the account from the
/// `master_account`.
///
pub struct UserTransaction {
    transaction: Transaction,
    signer: InMemorySigner,
    runtime: Runtime,
}

impl UserTransaction {
    /// Sign and execute the transaction
    pub fn submit(self) -> ExecutionResult {
        let res =
            (*self.runtime).borrow_mut().resolve_tx(self.transaction.sign(&self.signer)).unwrap();
        (*self.runtime).borrow_mut().process_all().unwrap();
        outcome_into_result(res, &self.runtime)
    }

    /// Create account for the receiver of the transaction.
    pub fn create_account(mut self) -> Self {
        self.transaction = self.transaction.create_account();
        self
    }

    /// Deploy Wasm binary
    pub fn deploy_contract(mut self, code: Vec<u8>) -> Self {
        self.transaction = self.transaction.deploy_contract(code);
        self
    }

    /// Execute contract call to receiver
    pub fn function_call(
        mut self,
        function_name: String,
        args: Vec<u8>,
        gas: Gas,
        deposit: Balance,
    ) -> Self {
        self.transaction = self.transaction.function_call(function_name, args, gas, deposit);
        self
    }

    /// Transfer deposit to receiver
    pub fn transfer(mut self, deposit: Balance) -> Self {
        self.transaction = self.transaction.transfer(deposit);
        self
    }

    /// Express interest in becoming a validator
    pub fn stake(mut self, stake: Balance, public_key: PublicKey) -> Self {
        self.transaction = self.transaction.stake(stake, public_key);
        self
    }

    /// Add access key, either FunctionCall or FullAccess
    pub fn add_key(mut self, public_key: PublicKey, access_key: AccessKey) -> Self {
        self.transaction = self.transaction.add_key(public_key, access_key);
        self
    }

    /// Delete an access key
    pub fn delete_key(mut self, public_key: PublicKey) -> Self {
        self.transaction = self.transaction.delete_key(public_key);
        self
    }

    /// Delete an account and send remaining balance to `beneficiary_id`
    pub fn delete_account(mut self, beneficiary_id: AccountId) -> Self {
        self.transaction = self.transaction.delete_account(String::from(beneficiary_id));
        self
    }
}

/// A user that can sign transactions.  It includes a signer and an account id.
pub struct UserAccount {
    runtime: Rc<RefCell<RuntimeStandalone>>,
    pub account_id: AccountId,
    pub signer: InMemorySigner,
}

impl Debug for UserAccount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserAccount").field("account_id", &self.account_id).finish()
    }
}

impl UserAccount {
    #[doc(hidden)]
    pub fn new(
        runtime: &Rc<RefCell<RuntimeStandalone>>,
        account_id: AccountId,
        signer: InMemorySigner,
    ) -> Self {
        let runtime = Rc::clone(runtime);
        Self { runtime, account_id, signer }
    }

    /// Returns a copy of the `account_id`
    pub fn account_id(&self) -> AccountId {
        self.account_id.clone()
    }
    /// Look up the account information on chain.
    pub fn account(&self) -> Option<Account> {
        (*self.runtime).borrow().view_account(self.account_id.as_str())
    }
    /// Transfer yoctoNear to another account
    pub fn transfer(&self, to: AccountId, deposit: Balance) -> ExecutionResult {
        self.submit_transaction(self.transaction(to).transfer(deposit))
    }

    /// Make a contract call.  `pending_tx` includes the receiver, the method to call as well as its arguments.
    /// Note: You will most likely not be using this method directly but rather the [`call!`](./macro.call.html) macro.
    pub fn function_call(
        &self,
        pending_tx: PendingContractTx,
        gas: Gas,
        deposit: Balance,
    ) -> ExecutionResult {
        self.call(pending_tx.receiver_id, &pending_tx.method, &pending_tx.args, gas, deposit)
    }

    pub fn call(
        &self,
        receiver_id: AccountId,
        method: &str,
        args: &[u8],
        gas: Gas,
        deposit: Balance,
    ) -> ExecutionResult {
        self.submit_transaction(self.transaction(receiver_id).function_call(
            method.to_string(),
            args.into(),
            gas,
            deposit,
        ))
    }

    /// Deploy a contract and create its account for `account_id`.
    /// Note: You will most likely not be using this method directly but rather the [`deploy!`](./macro.deploy.html) macro.
    pub fn deploy(
        &self,
        wasm_bytes: &[u8],
        account_id: AccountId,
        deposit: Balance,
    ) -> UserAccount {
        let signer =
            InMemorySigner::from_seed(account_id.as_str(), KeyType::ED25519, account_id.as_str());
        self.submit_transaction(
            self.transaction(account_id.clone())
                .create_account()
                .add_key(signer.public_key(), AccessKey::full_access())
                .transfer(deposit)
                .deploy_contract(wasm_bytes.to_vec()),
        )
        .assert_success();
        UserAccount::new(&self.runtime, account_id, signer)
    }

    /// Deploy a contract and in the same transaction call its initialization method.
    /// Note: You will most likely not be using this method directly but rather the [`deploy!`](./macro.deploy.html) macro.
    pub fn deploy_and_initialize(
        &self,
        wasm_bytes: &[u8],
        pending_tx: PendingContractTx,
        deposit: Balance,
        gas: Gas,
    ) -> UserAccount {
        self.deploy_and_init(
            wasm_bytes,
            pending_tx.receiver_id,
            &pending_tx.method,
            &pending_tx.args,
            deposit,
            gas,
        )
    }

    pub fn deploy_and_init(
        &self,
        wasm_bytes: &[u8],
        account_id: AccountId,
        method: &str,
        args: &[u8],
        deposit: Balance,
        gas: Gas,
    ) -> UserAccount {
        let signer =
            InMemorySigner::from_seed(account_id.as_str(), KeyType::ED25519, account_id.as_str());
        self.submit_transaction(
            self.transaction(account_id.clone())
                .create_account()
                .add_key(signer.public_key(), AccessKey::full_access())
                .transfer(deposit)
                .deploy_contract(wasm_bytes.to_vec())
                .function_call(method.to_string(), args.to_vec(), gas, 0),
        )
        .assert_success();
        UserAccount::new(&self.runtime, account_id, signer)
    }

    fn transaction(&self, receiver_id: AccountId) -> Transaction {
        let nonce = (*self.runtime)
            .borrow()
            .view_access_key(self.account_id.as_str(), &self.signer.public_key())
            .unwrap()
            .nonce
            + 1;
        Transaction::new(
            String::from(self.account_id()),
            self.signer.public_key(),
            String::from(receiver_id),
            nonce,
            CryptoHash::default(),
        )
    }

    /// Create a user transaction to `receiver_id` to be signed the current user
    pub fn create_transaction(&self, receiver_id: AccountId) -> UserTransaction {
        let transaction = self.transaction(receiver_id);
        let runtime = Rc::clone(&self.runtime);
        UserTransaction { transaction, signer: self.signer.clone(), runtime }
    }

    fn submit_transaction(&self, transaction: Transaction) -> ExecutionResult {
        let res = (*self.runtime).borrow_mut().resolve_tx(transaction.sign(&self.signer)).unwrap();
        (*self.runtime).borrow_mut().process_all().unwrap();
        outcome_into_result(res, &self.runtime)
    }

    /// Call a view method on a contract.
    /// Note: You will most likely not be using this method directly but rather the [`view!`](./macros.view.html) macro.
    pub fn view_method_call(&self, pending_tx: PendingContractTx) -> ViewResult {
        self.view(pending_tx.receiver_id, &pending_tx.method, &pending_tx.args)
    }

    pub fn view(&self, receiver_id: AccountId, method: &str, args: &[u8]) -> ViewResult {
        (*self.runtime).borrow().view_method_call(receiver_id.as_str(), method, args)
    }

    /// Creates a user and is signed by the `signer_user`
    pub fn create_user_from(
        &self,
        signer_user: &UserAccount,
        account_id: AccountId,
        amount: Balance,
    ) -> UserAccount {
        let signer =
            InMemorySigner::from_seed(account_id.as_str(), KeyType::ED25519, account_id.as_str());
        signer_user
            .submit_transaction(
                signer_user
                    .transaction(account_id.clone())
                    .create_account()
                    .add_key(signer.public_key(), AccessKey::full_access())
                    .transfer(amount),
            )
            .assert_success();
        UserAccount::new(&self.runtime, account_id, signer)
    }

    /// Create a new user where the signer is this user account
    pub fn create_user(&self, account_id: AccountId, amount: Balance) -> UserAccount {
        self.create_user_from(self, account_id, amount)
    }

    /// Returns a reference to a memory location of the standalone runtime.
    ///
    /// # Examples
    /// ```
    /// let master_account = near_sdk_sim::init_simulator(None);
    /// let runtime = master_account.borrow_runtime();
    ///
    /// // with use
    /// let _block = runtime.current_block();
    /// ```
    pub fn borrow_runtime(&self) -> Ref<RuntimeStandalone> {
        (*self.runtime).borrow()
    }

    /// Returns a mutable memory location to the standalone runtime.
    ///
    /// # Examples
    /// ```
    /// let master_account = near_sdk_sim::init_simulator(None);
    /// let mut runtime = master_account.borrow_runtime_mut();
    ///
    /// // with use
    /// runtime.produce_block().unwrap();
    /// ```
    pub fn borrow_runtime_mut(&self) -> RefMut<RuntimeStandalone> {
        (*self.runtime).borrow_mut()
    }
}

/// A account for a contract that includes a reference to the contract proxy and a user account
pub struct ContractAccount<T> {
    pub user_account: UserAccount,
    pub contract: T,
}

// TODO: find smarter way to respond to all `self.user_account` methods for a ContractAccount
impl<T> ContractAccount<T> {
    pub fn account_id(&self) -> AccountId {
        self.user_account.account_id()
    }
    pub fn account(&self) -> Option<Account> {
        self.user_account.account()
    }
}

/// The simulator takes an optional GenesisConfig, which sets up the fees and other settings.
/// It returns the `master_account` which can then create accounts and deploy contracts.
pub fn init_simulator(genesis_config: Option<GenesisConfig>) -> UserAccount {
    let (runtime, signer, root_account_id) = init_runtime(genesis_config);
    UserAccount::new(&Rc::new(RefCell::new(runtime)), root_account_id, signer)
}

/// Deploys a contract. Will either deploy or deploy and initialize a contract.
/// Returns a `ContractAccount<T>` where `T` is the first argument.
///
/// # Examples
///
/// The simplest example is deploying a contract without initializing it.
///
///
///  This example deploys and initializes the contract.
///
/// ```
/// # lazy_static_include::lazy_static_include_bytes! {
/// #    TOKEN_WASM_BYTES => "../examples/fungible-token/res/fungible_token.wasm",
/// # }
/// use near_sdk_sim::*;
/// use fungible_token::ContractContract;
/// use std::convert::TryInto;
/// use near_sdk::AccountId;
/// let master_account = near_sdk_sim::init_simulator(None);
/// let master_account_id: AccountId = master_account.account_id().try_into().unwrap();
/// let initial_balance = near_sdk_sim::to_yocto("35");
/// let contract = deploy! {
///   contract: ContractContract,
///   contract_id: "contract",
///   bytes: &TOKEN_WASM_BYTES,
///   signer_account: master_account,
///   init_method: new_default_meta(master_account_id, initial_balance.into())
/// };
/// ```
/// This example used the default values for the initial deposit to the new contract's account and gas for the contract call.
/// So it is the same as:
/// ```
/// # lazy_static_include::lazy_static_include_bytes! {
/// #    TOKEN_WASM_BYTES => "../examples/fungible-token/res/fungible_token.wasm",
/// # }
/// use fungible_token::ContractContract;
/// use near_sdk_sim::*;
/// use near_sdk::AccountId;
/// let master_account = near_sdk_sim::init_simulator(None);
/// let master_account_id: AccountId = master_account.account_id();
/// let initial_balance = near_sdk_sim::to_yocto("35");
/// let contract = deploy! {
///   contract: ContractContract,
///   contract_id: "contract",
///   bytes: &TOKEN_WASM_BYTES,
///   signer_account: master_account,
///   deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
///   gas: near_sdk_sim::DEFAULT_GAS,
///   init_method: new_default_meta(master_account_id, initial_balance.into()),
/// };
/// ```
#[macro_export]
macro_rules! deploy {
    ($contract: ident, $account_id:expr, $wasm_bytes: expr, $user:expr $(,)?) => {
        deploy!($contract, $account_id, $wasm_bytes, $user, near_sdk_sim::STORAGE_AMOUNT)
    };
    ($contract: ident, $account_id:expr, $wasm_bytes: expr, $user:expr, $deposit: expr $(,)?) => {
        near_sdk_sim::ContractAccount {
            user_account: $user.deploy($wasm_bytes, near_sdk::AccountId::new_unchecked($account_id.to_string()), $deposit),
            contract: $contract { account_id: near_sdk::AccountId::new_unchecked($account_id.to_string()) },
        }
    };
    ($contract: ident, $account_id:expr, $wasm_bytes: expr, $user_id:expr, $deposit:expr, $gas:expr, $method: ident, $($arg:expr),* $(,)?) => {
           {
               let __contract = $contract { account_id: near_sdk::AccountId::new_unchecked($account_id.to_string()) };
               near_sdk_sim::ContractAccount {
                   user_account: $user_id.deploy_and_initialize($wasm_bytes, __contract.$method($($arg),*), $deposit, $gas),
                   contract: __contract,
               }
           }
   };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr $(,)?) => {
      deploy!($contract, $account_id, $wasm_bytes, $user)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, deposit: $deposit: expr $(,)?) => {
        deploy!($contract, $account_id, $wasm_bytes, $user, $deposit)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, deposit: $deposit: expr, gas: $gas:expr, init_method: $method: ident($($arg:expr),*) $(,)?) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, $deposit, $gas, $method, $($arg),*)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, gas: $gas:expr, init_method: $method: ident($($arg:expr),*) $(,)?) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, near_sdk_sim::STORAGE_AMOUNT, $gas, $method, $($arg),*)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, deposit: $deposit: expr, init_method: $method: ident($($arg:expr),*) $(,)?) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, $deposit, near_sdk_sim::DEFAULT_GAS, $method, $($arg),*)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, init_method: $method: ident($($arg:expr),*) $(,)?) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, near_sdk_sim::STORAGE_AMOUNT, near_sdk_sim::DEFAULT_GAS, $method, $($arg),*)
    };
}

/// Makes a contract call to a [`ContractAccount`](./struct.ContractAccount.html) returning a [`ExecutionResult`](./struct.ExecutionResult.html).
///
///
/// # Examples:
///
/// ```
/// # lazy_static_include::lazy_static_include_bytes! {
/// #    TOKEN_WASM_BYTES => "../examples/fungible-token/res/fungible_token.wasm",
/// # }
/// # use near_sdk_sim::*;
/// # use fungible_token::ContractContract;
/// # use std::convert::TryInto;
/// # use near_sdk::AccountId;
/// # let master_account = near_sdk_sim::init_simulator(None);
/// # let master_account_id: AccountId = master_account.account_id().try_into().unwrap();
/// # let initial_balance = near_sdk_sim::to_yocto("35");
/// # let contract = deploy! {
/// # contract: ContractContract,
/// # contract_id: "contract",
/// # bytes: &TOKEN_WASM_BYTES,
/// # signer_account: master_account,
/// # deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
/// # gas: near_sdk_sim::DEFAULT_GAS,
/// # init_method: new_default_meta(master_account_id.clone(), initial_balance.into())
/// # };
/// use near_sdk_sim::to_yocto;
/// // Uses default values for gas and deposit.
/// let res = call!(
///      master_account,
///      contract.ft_transfer(master_account_id.clone(), to_yocto("100").into(), None)
///     );
/// // Equivalent to
/// let res = call!(
///     master_account,
///     contract.ft_transfer(master_account_id.clone(), to_yocto("100").into(), None),
///     0,
///     near_sdk_sim::DEFAULT_GAS
///    );
/// // Can also specify either deposit or gas
/// let res = call!(
///     master_account,
///     contract.ft_transfer(master_account_id.clone(), to_yocto("100").into(), None),
///     deposit = 0
///    );
/// let res = call!(
///     master_account,
///     contract.ft_transfer(master_account_id.clone(), to_yocto("100").into(), None),
///     gas = near_sdk_sim::DEFAULT_GAS
///    );
/// ```
#[macro_export]
macro_rules! call {
    ($signer:expr, $deposit: expr, $gas: expr, $contract: ident, $method:ident, $($arg:expr),*) => {
        $signer.function_call((&$contract).contract.$method($($arg),*), $gas, $deposit)
    };
    ($signer:expr, $contract: ident.$method:ident($($arg:expr),*), $deposit: expr, $gas: expr) => {
        call!($signer, $deposit, $gas, $contract, $method, $($arg),*)
    };
    ($signer:expr, $contract: ident.$method:ident($($arg:expr),*)) => {
        call!($signer, 0, near_sdk_sim::DEFAULT_GAS,  $contract, $method, $($arg),*)
    };
    ($signer:expr, $contract: ident.$method:ident($($arg:expr),*), gas=$gas_or_deposit: expr) => {
        call!($signer, 0, $gas_or_deposit, $contract, $method, $($arg),*)
    };
    ($signer:expr, $contract: ident.$method:ident($($arg:expr),*), deposit=$gas_or_deposit: expr) => {
        call!($signer, $gas_or_deposit, near_sdk_sim::DEFAULT_GAS, $contract, $method, $($arg),*)
    };
}

/// Calls a view method on the contract account.
///
/// Example:
/// ```
/// # lazy_static_include::lazy_static_include_bytes! {
/// #    TOKEN_WASM_BYTES => "../examples/fungible-token/res/fungible_token.wasm",
/// # }
/// # use near_sdk_sim::*;
/// # use fungible_token::ContractContract;
/// # use std::convert::TryInto;
/// # use near_sdk::AccountId;
/// # let master_account = near_sdk_sim::init_simulator(None);
/// # let master_account_id: AccountId = master_account.account_id().try_into().unwrap();
/// # let initial_balance = near_sdk_sim::to_yocto("35");
/// # let contract = deploy! {
/// # contract: ContractContract,
/// # contract_id: "contract",
/// # bytes: &TOKEN_WASM_BYTES,
/// # signer_account: master_account,
/// # deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
/// # gas: near_sdk_sim::DEFAULT_GAS,
/// # init_method: new_default_meta(master_account_id.clone(), initial_balance.into())
/// # };
/// let res = view!(contract.ft_balance_of(master_account_id));
/// ```
///
#[macro_export]
macro_rules! view {
    ($contract: ident.$method:ident($($arg:expr),*)) => {
        (&$contract).user_account.view_method_call((&$contract).contract.$method($($arg),*))
    };
}
