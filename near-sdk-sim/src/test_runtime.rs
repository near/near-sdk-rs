pub use crate::to_yocto;
use crate::{
    account::{AccessKey, Account},
    hash::CryptoHash,
    is_success,
    runtime::{GenesisConfig, RuntimeStandalone},
    transaction::{ExecutionOutcome, ExecutionStatus, Transaction},
    types::{AccountId, Balance, Gas},
};
use near_crypto::{InMemorySigner, KeyType, Signer};
use near_sdk::{serde_json, PendingContractTx};
use std::{cell::RefCell, rc::Rc};

pub const DEFAULT_GAS: u64 = 300_000_000_000_000;
pub const STORAGE_AMOUNT: u128 = 50_000_000_000_000_000_000_000_000;

pub type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;
pub type ViewResult = Result<(Vec<u8>, Vec<String>), Box<dyn std::error::Error>>;

pub fn outcome_into_result(outcome: ExecutionOutcome) -> TxResult {
    match outcome.status {
        ExecutionStatus::SuccessValue(_) => Ok(outcome),
        ExecutionStatus::Failure(_) => Err(outcome),
        ExecutionStatus::SuccessReceiptId(_) => panic!("Unresolved ExecutionOutcome run runtime.resolve(tx) to resolve the final outcome of tx"),
        ExecutionStatus::Unknown => unreachable!()
    }
}

pub struct User {
    runtime: Rc<RefCell<RuntimeStandalone>>,
    pub account_id: AccountId,
    pub signer: InMemorySigner,
}

impl User {
    pub fn new(
        runtime: &Rc<RefCell<RuntimeStandalone>>,
        account_id: AccountId,
        signer: InMemorySigner,
    ) -> Self {
        let runtime = Rc::clone(runtime);
        Self { runtime, account_id, signer }
    }

    pub fn account(&self) -> Option<Account> {
        (*self.runtime).borrow().view_account(&self.account_id)
    }

    pub fn call(&self, pending_tx: PendingContractTx, deposit: Balance, gas: Gas) -> TxResult {
        self.submit_transaction(self.transaction(pending_tx.receiver_id).function_call(
            pending_tx.method.to_string(),
            pending_tx.args,
            gas,
            deposit,
        ))
    }

    pub fn deploy_and_init(&self, wasm_bytes: &[u8], pending_tx: PendingContractTx) -> User {
        let signer = InMemorySigner::from_seed(
            &pending_tx.receiver_id.clone(),
            KeyType::ED25519,
            &pending_tx.receiver_id.clone(),
        );
        let account_id = pending_tx.receiver_id.clone();
        self.submit_transaction(
            self.transaction(pending_tx.receiver_id)
                .create_account()
                .add_key(signer.public_key(), AccessKey::full_access())
                .transfer(STORAGE_AMOUNT)
                .deploy_contract(wasm_bytes.to_vec())
                .function_call("new".to_string(), pending_tx.args, DEFAULT_GAS, 0),
        )
        .unwrap();
        User::new(&self.runtime, account_id, signer)
    }

    pub fn deploy(&self, wasm_bytes: &[u8], account_id: AccountId) -> User {
        let signer =
            InMemorySigner::from_seed(&account_id.clone(), KeyType::ED25519, &account_id.clone());
        self.submit_transaction(
            self.transaction(account_id.clone())
                .create_account()
                .add_key(signer.public_key(), AccessKey::full_access())
                .transfer(STORAGE_AMOUNT)
                .deploy_contract(wasm_bytes.to_vec()),
        )
        .unwrap();
        User::new(&self.runtime, account_id, signer)
    }

    pub fn transaction(&self, receiver_id: AccountId) -> Transaction {
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

    pub fn submit_transaction(&self, transaction: Transaction) -> TxResult {
        let res = (*self.runtime).borrow_mut().resolve_tx(transaction.sign(&self.signer)).unwrap();
        (*self.runtime).borrow_mut().process_all().unwrap();
        outcome_into_result(res)
    }

    pub fn view(&self, pending_tx: PendingContractTx) -> serde_json::Value {
        serde_json::from_slice(
            ((*self.runtime)
                .borrow()
                .view_method_call(&pending_tx.receiver_id, &pending_tx.method, &pending_tx.args)
                .unwrap()
                .0)
                .as_ref(),
        )
        .unwrap()
    }
}

pub struct TestRuntime {
    runtime: Rc<RefCell<RuntimeStandalone>>,
    pub root: User,
}

impl TestRuntime {
    pub fn new(runtime: RuntimeStandalone, signer: InMemorySigner, account_id: AccountId) -> Self {
        let runtime = Rc::new(RefCell::new(runtime));
        let root = User { runtime: Rc::clone(&runtime), account_id, signer };
        Self { runtime, root }
    }

    pub fn get_root(&self) -> &User {
        &self.root
    }

    pub fn create_user_from(
        &self,
        signer_user: &User,
        account_id: AccountId,
        amount: Balance,
    ) -> User {
        let signer = InMemorySigner::from_seed(&account_id.clone(), KeyType::ED25519, &account_id);
        signer_user
            .submit_transaction(
                signer_user
                    .transaction(account_id.clone())
                    .create_account()
                    .add_key(signer.public_key(), AccessKey::full_access())
                    .transfer(amount),
            )
            .unwrap();
        let account_id = account_id.clone();
        User { runtime: Rc::clone(&self.runtime), account_id, signer }
    }

    pub fn create_user(&self, account_id: AccountId, amount: Balance) -> User {
        self.create_user_from(&self.root, account_id, amount)
    }

    pub fn get_outcome(&self, hash: &CryptoHash) -> Option<ExecutionOutcome> {
        (*self.runtime).borrow().outcome(hash)
    }

    pub fn get_receipt_outcomes(
        &self,
        outcome: &ExecutionOutcome,
    ) -> Vec<Option<ExecutionOutcome>> {
        self.get_outcomes(&outcome.receipt_ids)
    }

    fn get_outcomes(&self, ids: &Vec<CryptoHash>) -> Vec<Option<ExecutionOutcome>> {
        ids.iter().map(|id| self.get_outcome(&id)).collect()
    }

    pub fn get_last_outcomes(&self) -> Vec<Option<ExecutionOutcome>> {
        self.get_outcomes(&(*self.runtime).borrow().last_outcomes)
    }

    pub fn find_errors(&self) -> Vec<Option<ExecutionOutcome>> {
        let mut res = self.get_last_outcomes();
        res.retain(|outcome| match outcome {
            Some(o) => !is_success(o),
            _ => false,
        });
        res
    }
}

pub fn init_test_runtime(genesis_config: Option<GenesisConfig>) -> TestRuntime {
    let mut genesis: GenesisConfig;
    if let Some(config) = genesis_config {
        genesis = config;
    } else {
        genesis = GenesisConfig::default();
        genesis.gas_limit = u64::max_value();
        genesis.runtime_config.wasm_config.limit_config.max_total_prepaid_gas = genesis.gas_limit;
    }
    let root_account_id = "root".to_string();
    let signer = genesis.init_root_signer(&root_account_id);
    let runtime = RuntimeStandalone::new_with_store(genesis);
    TestRuntime::new(runtime, signer, root_account_id)
}
