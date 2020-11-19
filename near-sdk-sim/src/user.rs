use crate::runtime::init_runtime;
pub use crate::to_yocto;
use crate::{
    account::{AccessKey, Account},
    hash::CryptoHash,
    outcome_into_result,
    runtime::{GenesisConfig, RuntimeStandalone},
    transaction::Transaction,
    types::{AccountId, Balance, Gas},
    ExecutionResult, ViewResult,
};
use near_crypto::{InMemorySigner, KeyType, PublicKey, Signer};
use near_sdk::PendingContractTx;
use std::{cell::RefCell, rc::Rc};

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
/// let account_id = "alice".to_string();
/// let transaction = master_account.create_transaction(account_id.clone());
/// // Creates a signer which contains a public key.
/// let signer = InMemorySigner::from_seed(&account_id, KeyType::ED25519, &account_id);
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
        method_name: String,
        args: Vec<u8>,
        gas: Gas,
        deposit: Balance,
    ) -> Self {
        self.transaction = self.transaction.function_call(method_name, args, gas, deposit);
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
        self.transaction = self.transaction.delete_account(beneficiary_id);
        self
    }
}

/// A user that can sign transactions.  It includes a signer and an account id.
pub struct UserAccount {
    runtime: Rc<RefCell<RuntimeStandalone>>,
    pub account_id: AccountId,
    pub signer: InMemorySigner,
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
        (*self.runtime).borrow().view_account(&self.account_id)
    }
    /// Transfer yoctoNear to another account
    pub fn transfer(&self, to: AccountId, deposit: Balance) -> ExecutionResult {
        self.submit_transaction(self.transaction(to).transfer(deposit))
    }

    /// Make a contract call.  `pending_tx` includes the receiver, the method to call as well as its arguments.
    /// Note: You will most likely not be using this method directly but rather the [`call!`](./macro.call.html) macro.
    pub fn call(
        &self,
        pending_tx: PendingContractTx,
        deposit: Balance,
        gas: Gas,
    ) -> ExecutionResult {
        self.submit_transaction(self.transaction(pending_tx.receiver_id).function_call(
            pending_tx.method.to_string(),
            pending_tx.args,
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
        let signer = InMemorySigner::from_seed(&account_id, KeyType::ED25519, &account_id);
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
    pub fn deploy_and_init(
        &self,
        wasm_bytes: &[u8],
        pending_tx: PendingContractTx,
        deposit: Balance,
        gas: Gas,
    ) -> UserAccount {
        let signer = InMemorySigner::from_seed(
            &pending_tx.receiver_id,
            KeyType::ED25519,
            &pending_tx.receiver_id,
        );
        let account_id = pending_tx.receiver_id.clone();
        self.submit_transaction(
            self.transaction(pending_tx.receiver_id)
                .create_account()
                .add_key(signer.public_key(), AccessKey::full_access())
                .transfer(deposit)
                .deploy_contract(wasm_bytes.to_vec())
                .function_call(pending_tx.method, pending_tx.args, gas, 0),
        )
        .assert_success();
        UserAccount::new(&self.runtime, account_id, signer)
    }

    fn transaction(&self, receiver_id: AccountId) -> Transaction {
        let nonce = (*self.runtime)
            .borrow()
            .view_access_key(&self.account_id, &self.signer.public_key())
            .unwrap()
            .nonce
            + 1;
        Transaction::new(
            self.account_id.clone(),
            self.signer.public_key(),
            receiver_id,
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
    pub fn view(&self, pending_tx: PendingContractTx) -> ViewResult {
        (*self.runtime).borrow().view_method_call(
            &pending_tx.receiver_id,
            &pending_tx.method,
            &pending_tx.args,
        )
    }

    /// Creates a user and is signed by the `signer_user`
    pub fn create_user_from(
        &self,
        signer_user: &UserAccount,
        account_id: AccountId,
        amount: Balance,
    ) -> UserAccount {
        let signer = InMemorySigner::from_seed(&account_id, KeyType::ED25519, &account_id);
        signer_user
            .submit_transaction(
                signer_user
                    .transaction(account_id.clone())
                    .create_account()
                    .add_key(signer.public_key(), AccessKey::full_access())
                    .transfer(amount),
            )
            .assert_success();
        UserAccount { runtime: Rc::clone(&self.runtime), account_id, signer }
    }

    /// Create a new user where the signer is this user account
    pub fn create_user(&self, account_id: AccountId, amount: Balance) -> UserAccount {
        self.create_user_from(&self, account_id, amount)
    }
}

/// A account for a contract that includes a reference to the contract proxy and a user account
pub struct ContractAccount<T> {
    pub user_account: UserAccount,
    pub contract: T,
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
/// # #[macro_use] extern crate near_sdk_sim;
/// # lazy_static::lazy_static! {
/// #    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../../examples/fungible-token/res/fungible_token.wasm").as_ref();
/// # }
/// use fungible_token::FungibleTokenContract;
/// let master_account = near_sdk_sim::init_simulator(None);
/// let initial_balance = near_sdk_sim::to_yocto("35");
/// let contract = deploy! {
///   contract: FungibleTokenContract,
///   contract_id: "contract",
///   bytes: &TOKEN_WASM_BYTES,
///   signer_account: master_account,
///   init_method: new(master_account.account_id(), initial_balance.into())
/// };
/// ```
/// This example used the default values for the initial deposit to the new contract's account and gas for the contract call.
/// So it is the same as:
/// ```
/// # #[macro_use] extern crate near_sdk_sim;
/// # lazy_static::lazy_static! {
/// #    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../../examples/fungible-token/res/fungible_token.wasm").as_ref();
/// # }
/// use fungible_token::FungibleTokenContract;
/// let master_account = near_sdk_sim::init_simulator(None);
/// let initial_balance = near_sdk_sim::to_yocto("35");
/// let contract = deploy! {
/// contract: FungibleTokenContract,
///   contract_id: "contract",
///   bytes: &TOKEN_WASM_BYTES,
///   signer_account: master_account,
///   deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
///   gas: near_sdk_sim::DEFAULT_GAS,
///   init_method: new(master_account.account_id(), initial_balance.into())
/// };
/// ```
#[doc(inline)]
#[macro_export]
macro_rules! deploy {
    ($contract: ident, $account_id:expr, $wasm_bytes: expr, $user:expr) => {
        deploy!($contract, $account_id, $wasm_bytes, $user, near_sdk_sim::STORAGE_AMOUNT)
    };
    ($contract: ident, $account_id:expr, $wasm_bytes: expr, $user:expr, $deposit: expr) => {
        near_sdk_sim::ContractAccount {
            user_account: $user.deploy($wasm_bytes, $account_id.to_string(), $deposit),
            contract: $contract { account_id: $account_id.to_string() },
        }
    };
    ($contract: ident, $account_id:expr, $wasm_bytes: expr, $user_id:expr, $deposit:expr, $gas:expr, $method: ident, $($arg:expr),* ) => {
           {
               let __contract = $contract { account_id: $account_id.to_string() };
               near_sdk_sim::ContractAccount {
                   user_account: $user_id.deploy_and_init($wasm_bytes, __contract.$method($($arg),*), $deposit, $gas),
                   contract: __contract,
               }
           }
   };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr) => {
      deploy!($contract, $account_id, $wasm_bytes, $user)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, deposit: $deposit: expr) => {
        deploy!($contract, $account_id, $wasm_bytes, $user, $deposit)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, deposit: $deposit: expr, gas: $gas:expr, init_method: $method: ident($($arg:expr),*) ) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, $deposit, $gas, $method, $($arg),*)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, gas: $gas:expr, init_method: $method: ident($($arg:expr),*) ) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, near_sdk_sim::STORAGE_AMOUNT, $gas, $method, $($arg),*)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, deposit: $deposit: expr, init_method: $method: ident($($arg:expr),*) ) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, $deposit, near_sdk_sim::DEFAULT_GAS, $method, $($arg),*)
    };
    (contract: $contract: ident, contract_id: $account_id:expr, bytes: $wasm_bytes: expr, signer_account: $user:expr, init_method: $method: ident($($arg:expr),+) ) => {
       deploy!($contract, $account_id, $wasm_bytes, $user, near_sdk_sim::STORAGE_AMOUNT, near_sdk_sim::DEFAULT_GAS, $method, $($arg),*)
    };
}

/// Makes a contract call to a [`ContractAccount`](./struct.ContractAccount.html) returning a [`ExecutionResult`](./struct.ExecutionResult.html).
///
///
/// # Examples:
///
/// ```
/// # #[macro_use] extern crate near_sdk_sim;
/// # lazy_static::lazy_static! {
/// #    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../../examples/fungible-token/res/fungible_token.wasm").as_ref();
/// # }
/// # use fungible_token::FungibleTokenContract;
/// # let master_account = near_sdk_sim::init_simulator(None);
/// # let initial_balance = near_sdk_sim::to_yocto("35");
/// # let contract = deploy! {
/// # contract: FungibleTokenContract,
/// # contract_id: "contract",
/// # bytes: &TOKEN_WASM_BYTES,
/// # signer_account: master_account,
/// # deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
/// # gas: near_sdk_sim::DEFAULT_GAS,
/// # init_method: new(master_account.account_id(), initial_balance.into())
/// # };
/// use near_sdk_sim::to_yocto;
/// // Uses default values for gas and deposit.
/// let res = call!(
///      master_account,
///      contract.transfer(master_account.account_id(), to_yocto("100").into())
///     );
/// // Equivalent to
/// let res = call!(
///     master_account,
///     contract.transfer(master_account.account_id(), to_yocto("100").into()),
///     0,
///     near_sdk_sim::DEFAULT_GAS
///    );
/// // Can also specify either deposit or gas
/// let res = call!(
///     master_account,
///     contract.transfer(master_account.account_id(), to_yocto("100").into()),
///     deposit = 0
///    );
/// let res = call!(
///     master_account,
///     contract.transfer(master_account.account_id(), to_yocto("100").into()),
///     gas = near_sdk_sim::DEFAULT_GAS
///    );
/// ```
#[macro_export]
macro_rules! call {
    ($signer:expr, $deposit: expr, $gas: expr, $contract: ident, $method:ident, $($arg:expr),*) => {
        $signer.call((&$contract).contract.$method($($arg),*), $deposit, $gas)
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
/// # #[macro_use] extern crate near_sdk_sim;
/// # lazy_static::lazy_static! {
/// #    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../../examples/fungible-token/res/fungible_token.wasm").as_ref();
/// # }
/// # use fungible_token::FungibleTokenContract;
/// # let master_account = near_sdk_sim::init_simulator(None);
/// # let initial_balance = near_sdk_sim::to_yocto("35");
/// # let contract = deploy! {
/// # contract: FungibleTokenContract,
/// # contract_id: "contract",
/// # bytes: &TOKEN_WASM_BYTES,
/// # signer_account: master_account,
/// # deposit: near_sdk_sim::STORAGE_AMOUNT, // Deposit required to cover contract storage.
/// # gas: near_sdk_sim::DEFAULT_GAS,
/// # init_method: new(master_account.account_id(), initial_balance.into())
/// # };
/// let res = view!(contract.get_balance(master_account.account_id()));
/// ```
///
#[macro_export]
macro_rules! view {
    ($contract: ident.$method:ident($($arg:expr),*)) => {
        (&$contract).user_account.view((&$contract).contract.$method($($arg),*))
    };
}
