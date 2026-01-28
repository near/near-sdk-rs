//! Contract executor - wraps near-vm-runner for single contract execution

use crate::error::{SimError, SimResult};
use crate::outcome::{ExecutionStatus, ReceiptOutcome};
use crate::state::GlobalState;

use near_account_id::AccountId;
use near_gas::NearGas;
use near_parameters::vm::Config as VmConfig;
use near_parameters::{RuntimeConfigStore, RuntimeFeesConfig};
use near_primitives_core::version::PROTOCOL_VERSION;
use near_token::NearToken;
use near_vm_runner::internal::VMKindExt;
use near_vm_runner::logic::mocks::mock_external::{MockAction, MockedExternal};
use near_vm_runner::logic::types::PromiseResult;
use near_vm_runner::logic::{ReturnData, VMContext};

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Unique identifier for a receipt
pub type ReceiptId = u64;

/// A receipt representing a contract call
#[derive(Debug, Clone)]
pub struct Receipt {
    pub id: ReceiptId,
    pub predecessor: AccountId,
    pub receiver: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub deposit: NearToken,
    pub gas: NearGas,
    /// Receipts this one depends on (for callbacks)
    pub depends_on: Vec<ReceiptId>,
}

/// How a receipt completed
#[derive(Debug)]
pub enum Completion {
    /// Returned a value
    Value(Vec<u8>),
    /// Failed
    Failed,
    /// Forward result to another receipt (returned a promise)
    Forward(ReceiptId),
}

/// Result of executing a single receipt
pub struct ExecutionResult {
    pub outcome: ReceiptOutcome,
    pub new_receipts: Vec<Receipt>,
    pub storage_changes: HashMap<Vec<u8>, Vec<u8>>,
    pub completion: Completion,
}

/// Execute a single contract call
pub fn execute(
    state: &GlobalState,
    receipt: &Receipt,
    promise_results: Vec<PromiseResult>,
    block_height: u64,
    block_timestamp: u64,
    epoch_height: u64,
    next_id: &mut ReceiptId,
) -> SimResult<ExecutionResult> {
    let code = state
        .get_code(&receipt.receiver)
        .ok_or_else(|| SimError::contract_not_found(receipt.receiver.clone()))?;

    let storage = state.get_storage(&receipt.receiver).cloned().unwrap_or_default();

    let mut ext = MockedExternal::with_code_and_hash(*code.hash(), (*code).clone());
    ext.fake_trie = storage;

    let context = VMContext {
        current_account_id: receipt.receiver.clone(),
        signer_account_id: receipt.predecessor.clone(),
        signer_account_pk: vec![0u8; 33],
        predecessor_account_id: receipt.predecessor.clone(),
        input: Rc::from(receipt.args.as_slice()),
        block_height,
        block_timestamp,
        epoch_height,
        account_balance: state.get_balance(&receipt.receiver),
        account_locked_balance: NearToken::from_near(0),
        storage_usage: state.get(&receipt.receiver).map_or(0, |a| a.calculate_storage_usage()),
        attached_deposit: receipt.deposit,
        prepaid_gas: near_primitives_core::types::Gas::from_gas(receipt.gas.as_gas()),
        random_seed: vec![0u8; 32],
        view_config: None,
        output_data_receivers: vec![],
        promise_results: Arc::from(promise_results),
        refund_to_account_id: receipt.predecessor.clone(),
        account_contract: near_primitives_core::account::AccountContract::None,
    };

    let vm_config = default_vm_config();
    let fees_config = default_fees_config();
    let runtime =
        vm_config.vm_kind.runtime(vm_config.clone()).expect("Wasmtime runtime should be available");

    let gas_counter = context.make_gas_counter(&vm_config);
    let prepared = runtime.prepare(&ext, None, gas_counter, &receipt.method);

    match prepared.run(&mut ext, &context, fees_config) {
        Ok(outcome) => {
            let (status, completion, return_value) = match &outcome.aborted {
                Some(err) => {
                    let msg = format!("{:?}", err);
                    if msg.contains("Panic") {
                        (ExecutionStatus::Panic(msg), Completion::Failed, None)
                    } else {
                        (ExecutionStatus::Failure(msg), Completion::Failed, None)
                    }
                }
                None => {
                    let (new_receipts, action_to_id) = extract_receipts(&ext, receipt, next_id);

                    let (completion, return_value) = match &outcome.return_data {
                        ReturnData::Value(data) => {
                            (Completion::Value(data.clone()), Some(data.clone()))
                        }
                        ReturnData::ReceiptIndex(idx) => {
                            if let Some(&target) = action_to_id.get(idx) {
                                (Completion::Forward(target), None)
                            } else {
                                (Completion::Value(vec![]), None)
                            }
                        }
                        ReturnData::None => (Completion::Value(vec![]), None),
                    };

                    return Ok(ExecutionResult {
                        outcome: ReceiptOutcome {
                            predecessor: receipt.predecessor.clone(),
                            receiver: receipt.receiver.clone(),
                            method: receipt.method.clone(),
                            args: receipt.args.clone(),
                            deposit: receipt.deposit,
                            gas_used: NearGas::from_gas(outcome.burnt_gas.as_gas()),
                            status: ExecutionStatus::Success,
                            logs: outcome.logs,
                            return_value,
                        },
                        new_receipts,
                        storage_changes: ext.fake_trie,
                        completion,
                    });
                }
            };

            Ok(ExecutionResult {
                outcome: ReceiptOutcome {
                    predecessor: receipt.predecessor.clone(),
                    receiver: receipt.receiver.clone(),
                    method: receipt.method.clone(),
                    args: receipt.args.clone(),
                    deposit: receipt.deposit,
                    gas_used: NearGas::from_gas(outcome.burnt_gas.as_gas()),
                    status,
                    logs: outcome.logs,
                    return_value,
                },
                new_receipts: vec![],
                storage_changes: ext.fake_trie,
                completion,
            })
        }
        Err(err) => Ok(ExecutionResult {
            outcome: ReceiptOutcome {
                predecessor: receipt.predecessor.clone(),
                receiver: receipt.receiver.clone(),
                method: receipt.method.clone(),
                args: receipt.args.clone(),
                deposit: receipt.deposit,
                gas_used: NearGas::from_gas(0),
                status: ExecutionStatus::Failure(format!("{:?}", err)),
                logs: vec![],
                return_value: None,
            },
            new_receipts: vec![],
            storage_changes: ext.fake_trie,
            completion: Completion::Failed,
        }),
    }
}

/// Extract new receipts from the action log
fn extract_receipts(
    ext: &MockedExternal,
    parent: &Receipt,
    next_id: &mut ReceiptId,
) -> (Vec<Receipt>, HashMap<u64, ReceiptId>) {
    let mut action_to_id: HashMap<u64, ReceiptId> = HashMap::new();
    let mut receipts = Vec::new();

    // First pass: assign IDs to all CreateReceipt actions
    for (idx, action) in ext.action_log.iter().enumerate() {
        if matches!(action, MockAction::CreateReceipt { .. }) {
            let id = *next_id;
            *next_id += 1;
            action_to_id.insert(idx as u64, id);
        }
    }

    // Second pass: build receipts by matching CreateReceipt with FunctionCallWeight
    for (idx, action) in ext.action_log.iter().enumerate() {
        if let MockAction::CreateReceipt { receipt_indices, receiver_id } = action {
            let id = action_to_id[&(idx as u64)];

            // Find the FunctionCallWeight that targets this receipt
            let call_info = ext.action_log.iter().find_map(|a| {
                if let MockAction::FunctionCallWeight {
                    receipt_index,
                    method_name,
                    args,
                    attached_deposit,
                    prepaid_gas,
                    ..
                } = a
                {
                    if *receipt_index as usize == idx {
                        return Some((method_name, args, *attached_deposit, *prepaid_gas));
                    }
                }
                None
            });

            if let Some((method, args, deposit, gas)) = call_info {
                receipts.push(Receipt {
                    id,
                    predecessor: parent.receiver.clone(),
                    receiver: receiver_id.clone(),
                    method: String::from_utf8_lossy(method).into(),
                    args: args.clone(),
                    deposit,
                    gas: if gas.as_gas() > 0 {
                        NearGas::from_gas(gas.as_gas())
                    } else {
                        parent.gas
                    },
                    depends_on: receipt_indices
                        .iter()
                        .filter_map(|i| action_to_id.get(i).copied())
                        .collect(),
                });
            }
        }
    }

    (receipts, action_to_id)
}

fn default_vm_config() -> Arc<VmConfig> {
    let store = RuntimeConfigStore::test();
    let config = store.get_config(PROTOCOL_VERSION).wasm_config.as_ref().to_owned();
    Arc::new(VmConfig { vm_kind: near_parameters::vm::VMKind::Wasmtime, ..config })
}

fn default_fees_config() -> Arc<RuntimeFeesConfig> {
    Arc::new(RuntimeFeesConfig::test())
}
