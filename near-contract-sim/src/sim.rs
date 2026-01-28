//! Main ContractSim runtime - lightweight multi-contract testing

use crate::error::{SimError, SimResult};
use crate::executor::{self, Completion, Receipt, ReceiptId};
use crate::outcome::{CallOutcome, ExecutionStatus, MockResponse, ReceiptOutcome};
use crate::state::GlobalState;

use near_account_id::AccountId;
use near_gas::NearGas;
use near_token::NearToken;
use near_vm_runner::logic::types::PromiseResult;

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// Mock handler function type
type MockHandler = Box<dyn Fn(&str, &[u8]) -> MockResponse + Send + Sync>;

/// Lightweight multi-contract testing runtime
pub struct ContractSim {
    state: GlobalState,
    mocks: HashMap<AccountId, MockHandler>,
    block_height: u64,
    block_timestamp: u64,
    epoch_height: u64,
    default_gas: NearGas,
}

impl Default for ContractSim {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractSim {
    pub fn new() -> Self {
        Self {
            state: GlobalState::new(),
            mocks: HashMap::new(),
            block_height: 1,
            block_timestamp: 0,
            epoch_height: 1,
            default_gas: NearGas::from_tgas(300),
        }
    }

    // =========================================================================
    // Account & Contract Management
    // =========================================================================

    pub fn create_account(&mut self, account_id: &str, balance: NearToken) -> SimResult<()> {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        self.state.set_balance(&account_id, balance);
        Ok(())
    }

    pub fn deploy(&mut self, account_id: &str, code: impl Into<Vec<u8>>) -> SimResult<()> {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        self.state.deploy(&account_id, code.into());
        Ok(())
    }

    pub fn deploy_file(&mut self, account_id: &str, path: impl AsRef<Path>) -> SimResult<()> {
        let code = std::fs::read(path.as_ref()).map_err(|e| SimError::IoError {
            message: format!("Failed to read WASM file: {}", e),
        })?;
        self.deploy(account_id, code)
    }

    pub fn balance(&self, account_id: &str) -> SimResult<NearToken> {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        Ok(self.state.get_balance(&account_id))
    }

    pub fn has_contract(&self, account_id: &str) -> SimResult<bool> {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        Ok(self.state.has_contract(&account_id))
    }

    pub fn accounts(&self) -> Vec<String> {
        self.state.account_ids().map(|id| id.to_string()).collect()
    }

    // =========================================================================
    // Mocking
    // =========================================================================

    /// Register a mock handler for all methods on an account
    pub fn mock<F>(&mut self, account_id: &str, handler: F) -> SimResult<()>
    where
        F: Fn(&str, &[u8]) -> MockResponse + Send + Sync + 'static,
    {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        self.mocks.insert(account_id, Box::new(handler));
        Ok(())
    }

    /// Register a mock that returns a fixed JSON response for any method
    pub fn mock_json<T: serde::Serialize>(
        &mut self,
        account_id: &str,
        response: &T,
    ) -> SimResult<()> {
        let bytes = serde_json::to_vec(response)
            .map_err(|e| SimError::DeserializationError { message: e.to_string() })?;
        self.mock(account_id, move |_, _| MockResponse::Success(bytes.clone()))
    }

    // =========================================================================
    // Time Control
    // =========================================================================

    pub fn advance_block(&mut self) {
        self.block_height += 1;
        self.block_timestamp += 1_000_000_000;
    }

    pub fn advance_blocks(&mut self, n: u64) {
        self.block_height += n;
        self.block_timestamp += n * 1_000_000_000;
    }

    pub fn set_block_height(&mut self, height: u64) {
        self.block_height = height;
    }

    pub fn set_block_timestamp(&mut self, timestamp: u64) {
        self.block_timestamp = timestamp;
    }

    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    pub fn block_timestamp(&self) -> u64 {
        self.block_timestamp
    }

    // =========================================================================
    // Storage Access
    // =========================================================================

    pub fn storage_read(&self, account_id: &str, key: &[u8]) -> SimResult<Option<Vec<u8>>> {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        Ok(self.state.get_storage(&account_id).and_then(|s| s.get(key).cloned()))
    }

    pub fn storage_dump(&self, account_id: &str) -> SimResult<HashMap<Vec<u8>, Vec<u8>>> {
        let account_id: AccountId = account_id
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: account_id.to_string() })?;
        Ok(self.state.get_storage(&account_id).cloned().unwrap_or_default())
    }

    pub fn storage_keys(&self, account_id: &str) -> SimResult<Vec<String>> {
        let storage = self.storage_dump(account_id)?;
        Ok(storage.keys().map(|k| String::from_utf8_lossy(k).to_string()).collect())
    }

    // =========================================================================
    // Contract Execution
    // =========================================================================

    pub fn call(
        &mut self,
        signer: &str,
        receiver: &str,
        method: &str,
        args: &[u8],
    ) -> SimResult<CallOutcome> {
        self.call_with_options(
            signer,
            receiver,
            method,
            args,
            NearToken::from_near(0),
            self.default_gas,
        )
    }

    pub fn call_json<T: serde::Serialize>(
        &mut self,
        signer: &str,
        receiver: &str,
        method: &str,
        args: &T,
    ) -> SimResult<CallOutcome> {
        let args = serde_json::to_vec(args)
            .map_err(|e| SimError::DeserializationError { message: e.to_string() })?;
        self.call(signer, receiver, method, &args)
    }

    pub fn call_with_deposit(
        &mut self,
        signer: &str,
        receiver: &str,
        method: &str,
        args: &[u8],
        deposit: NearToken,
    ) -> SimResult<CallOutcome> {
        self.call_with_options(signer, receiver, method, args, deposit, self.default_gas)
    }

    pub fn call_with_options(
        &mut self,
        signer: &str,
        receiver: &str,
        method: &str,
        args: &[u8],
        deposit: NearToken,
        gas: NearGas,
    ) -> SimResult<CallOutcome> {
        let signer: AccountId =
            signer.parse().map_err(|_| SimError::InvalidAccountId { id: signer.to_string() })?;
        let receiver: AccountId = receiver
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: receiver.to_string() })?;

        let initial = Receipt {
            id: 0,
            predecessor: signer,
            receiver,
            method: method.to_string(),
            args: args.to_vec(),
            deposit,
            gas,
            depends_on: vec![],
        };

        self.execute_all(initial)
    }

    pub fn view(&self, receiver: &str, method: &str, args: &[u8]) -> SimResult<CallOutcome> {
        let receiver: AccountId = receiver
            .parse()
            .map_err(|_| SimError::InvalidAccountId { id: receiver.to_string() })?;

        let receipt = Receipt {
            id: 0,
            predecessor: receiver.clone(),
            receiver,
            method: method.to_string(),
            args: args.to_vec(),
            deposit: NearToken::from_near(0),
            gas: self.default_gas,
            depends_on: vec![],
        };

        let mut next_id = 1;
        let result = executor::execute(
            &self.state,
            &receipt,
            vec![],
            self.block_height,
            self.block_timestamp,
            self.epoch_height,
            &mut next_id,
        )?;

        Ok(CallOutcome::new(vec![result.outcome]))
    }

    pub fn view_json<T: serde::Serialize>(
        &self,
        receiver: &str,
        method: &str,
        args: &T,
    ) -> SimResult<CallOutcome> {
        let args = serde_json::to_vec(args)
            .map_err(|e| SimError::DeserializationError { message: e.to_string() })?;
        self.view(receiver, method, &args)
    }

    // =========================================================================
    // Core Execution Loop
    // =========================================================================

    /// Execute all receipts until completion
    fn execute_all(&mut self, initial: Receipt) -> SimResult<CallOutcome> {
        let mut pending: Vec<Receipt> = vec![initial];
        let mut results: HashMap<ReceiptId, PromiseResult> = HashMap::new();
        let mut outcomes: Vec<ReceiptOutcome> = Vec::new();
        let mut next_id: ReceiptId = 1;

        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000;

        while let Some(receipt) = self.pop_ready(&mut pending, &results) {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(SimError::ExecutionError {
                    account_id: "system".parse().unwrap(),
                    method: "execute_all".to_string(),
                    message: "Exceeded maximum iterations".to_string(),
                });
            }

            let receipt_id = receipt.id;

            // Collect promise results from dependencies
            let promise_results: Vec<PromiseResult> = receipt
                .depends_on
                .iter()
                .filter_map(|id| results.get(id).map(clone_promise_result))
                .collect();

            // Check for mock
            if let Some(handler) = self.mocks.get(&receipt.receiver) {
                let response = handler(&receipt.method, &receipt.args);
                let (outcome, completion) = execute_mock(&receipt, response);
                self.handle_completion(receipt_id, completion, &mut pending, &mut results);
                outcomes.push(outcome);
                continue;
            }

            // Execute real contract
            let exec_result = executor::execute(
                &self.state,
                &receipt,
                promise_results,
                self.block_height,
                self.block_timestamp,
                self.epoch_height,
                &mut next_id,
            )?;

            // Update storage
            *self.state.get_storage_mut(&receipt.receiver) = exec_result.storage_changes;

            // Add new receipts
            pending.extend(exec_result.new_receipts);

            // Handle completion
            self.handle_completion(receipt_id, exec_result.completion, &mut pending, &mut results);
            outcomes.push(exec_result.outcome);
        }

        // Check for deadlock
        if !pending.is_empty() {
            return Err(SimError::ExecutionError {
                account_id: "system".parse().unwrap(),
                method: "execute_all".to_string(),
                message: "Deadlock: receipts waiting for dependencies that will never complete"
                    .to_string(),
            });
        }

        Ok(CallOutcome::new(outcomes))
    }

    /// Pop the next ready receipt (all dependencies satisfied)
    fn pop_ready(
        &self,
        pending: &mut Vec<Receipt>,
        results: &HashMap<ReceiptId, PromiseResult>,
    ) -> Option<Receipt> {
        let idx =
            pending.iter().position(|r| r.depends_on.iter().all(|id| results.contains_key(id)))?;
        Some(pending.swap_remove(idx))
    }

    /// Handle receipt completion: store result or rewrite dependencies on forward
    fn handle_completion(
        &self,
        id: ReceiptId,
        completion: Completion,
        pending: &mut Vec<Receipt>,
        results: &mut HashMap<ReceiptId, PromiseResult>,
    ) {
        match completion {
            Completion::Value(data) => {
                results.insert(id, PromiseResult::Successful(Rc::from(data.as_slice())));
            }
            Completion::Failed => {
                results.insert(id, PromiseResult::Failed);
            }
            Completion::Forward(target_id) => {
                // Rewrite dependencies: anyone waiting for `id` now waits for `target_id`
                for receipt in pending.iter_mut() {
                    for dep in receipt.depends_on.iter_mut() {
                        if *dep == id {
                            *dep = target_id;
                        }
                    }
                }
            }
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn clone_promise_result(pr: &PromiseResult) -> PromiseResult {
    match pr {
        PromiseResult::NotReady => PromiseResult::NotReady,
        PromiseResult::Successful(data) => PromiseResult::Successful(Rc::clone(data)),
        PromiseResult::Failed => PromiseResult::Failed,
    }
}

fn execute_mock(receipt: &Receipt, response: MockResponse) -> (ReceiptOutcome, Completion) {
    let (status, return_value, completion) = match response {
        MockResponse::Success(data) => {
            (ExecutionStatus::Success, Some(data.clone()), Completion::Value(data))
        }
        MockResponse::Failure(msg) => (ExecutionStatus::Failure(msg), None, Completion::Failed),
        MockResponse::Panic(msg) => (ExecutionStatus::Panic(msg), None, Completion::Failed),
    };

    (
        ReceiptOutcome {
            predecessor: receipt.predecessor.clone(),
            receiver: receipt.receiver.clone(),
            method: receipt.method.clone(),
            args: receipt.args.clone(),
            deposit: receipt.deposit,
            gas_used: NearGas::from_gas(0),
            status,
            logs: vec![],
            return_value,
        },
        completion,
    )
}

// =========================================================================
// Builder API
// =========================================================================

pub struct CallBuilder<'a> {
    sim: &'a mut ContractSim,
    receiver: String,
    method: String,
    signer: Option<String>,
    args: Vec<u8>,
    deposit: NearToken,
    gas: NearGas,
}

impl ContractSim {
    pub fn call_builder(&mut self, receiver: &str, method: &str) -> CallBuilder<'_> {
        CallBuilder {
            sim: self,
            receiver: receiver.to_string(),
            method: method.to_string(),
            signer: None,
            args: vec![],
            deposit: NearToken::from_near(0),
            gas: NearGas::from_tgas(300),
        }
    }
}

impl<'a> CallBuilder<'a> {
    pub fn signer(mut self, signer: &str) -> Self {
        self.signer = Some(signer.to_string());
        self
    }

    pub fn args(mut self, args: &[u8]) -> Self {
        self.args = args.to_vec();
        self
    }

    pub fn args_json<T: serde::Serialize>(mut self, args: &T) -> Self {
        self.args = serde_json::to_vec(args).unwrap();
        self
    }

    pub fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = deposit;
        self
    }

    pub fn deposit_near(mut self, near: u128) -> Self {
        self.deposit = NearToken::from_near(near);
        self
    }

    pub fn deposit_yocto(mut self, yocto: u128) -> Self {
        self.deposit = NearToken::from_yoctonear(yocto);
        self
    }

    pub fn gas(mut self, gas: NearGas) -> Self {
        self.gas = gas;
        self
    }

    pub fn gas_tgas(mut self, tgas: u64) -> Self {
        self.gas = NearGas::from_tgas(tgas);
        self
    }

    pub fn execute(self) -> SimResult<CallOutcome> {
        let signer = self.signer.unwrap_or_else(|| self.receiver.clone());
        self.sim.call_with_options(
            &signer,
            &self.receiver,
            &self.method,
            &self.args,
            self.deposit,
            self.gas,
        )
    }
}
