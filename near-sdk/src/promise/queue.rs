use std::{cell::RefCell, collections::BTreeMap};

use super::PromiseAction;
use crate::{env, AccountId, Gas, PromiseIndex};

thread_local! {
    static QUEUED_PROMISES: RefCell<Vec<PromiseQueueEvent>> = RefCell::new(Vec::new());
}

#[derive(Clone, Copy, Debug)]
pub struct QueueIndex(usize);

#[non_exhaustive]
#[derive(Debug)]
pub enum PromiseQueueEvent {
    CreateBatch { account_id: AccountId },
    BatchAnd { promise_a: QueueIndex, promise_b: QueueIndex },
    BatchThen { account_id: AccountId, following_index: QueueIndex },
    Action { index: QueueIndex, action: PromiseAction },
    Return { index: QueueIndex },
}

pub(super) fn queue_promise_event(event: PromiseQueueEvent) -> QueueIndex {
    QUEUED_PROMISES.with(|q| {
        let mut q = q.borrow_mut();
        q.push(event);
        QueueIndex(q.len() - 1)
    })
}

fn calc_gas_per_unspecified(events: &[PromiseQueueEvent]) -> Gas {
    // let mut remaining_gas = env::prepaid_gas() - env::used_gas();

    // // Only take a factor of the gas remaining because of constant fees
    // // TODO remove this in the future, this is a horrible pattern
    // remaining_gas = (remaining_gas / 10) * 6;

    // subtract any defined amounts and count number of unspecified
    let mut count_remaining = 0;
    for event in events.iter() {
        match event {
            PromiseQueueEvent::Action { action: PromiseAction::FunctionCall { .. }, .. } => {
                count_remaining += 1;
                // if let Some(specified) = gas {
                //     // Gas was specified, remove this from pool of funds
                //     remaining_gas -= *specified;
                // } else {
                //     count_remaining += 1;
                // }
            }
            _ => (),
        }
    }

    if count_remaining == 0 {
        Gas(0)
    } else {
        env::prepaid_gas() / (count_remaining + 1)
        // 	// Split remaining gas among the count of unspecified gas
        //     remaining_gas / count_remaining
    }
}

/// Schedules queued function calls, which will split remaining gas
pub fn schedule_queued_promises() {
    QUEUED_PROMISES.with(|q| {
        let mut queue = q.borrow_mut();

        // Would be ideal if this is calculated only if an unspecified gas found
        let function_call_gas = calc_gas_per_unspecified(&queue);

        let mut lookup = BTreeMap::<usize, PromiseIndex>::new();

        for (i, event) in queue.iter().enumerate() {
            match event {
                PromiseQueueEvent::CreateBatch { account_id } => {
                    let promise_idx = crate::env::promise_batch_create(account_id);
                    lookup.insert(i, promise_idx);
                }
                PromiseQueueEvent::BatchAnd { promise_a, promise_b } => {
                    //* indices guaranteed to be in lookup as the queue index would be used
                    // 	to create the `and` promise.
                    let a = lookup[&promise_a.0];
                    let b = lookup[&promise_b.0];
                    let promise_idx = crate::env::promise_and(&[a, b]);
                    lookup.insert(i, promise_idx);
                }
                PromiseQueueEvent::BatchThen { account_id, following_index } => {
                    let following = lookup[&following_index.0];
                    let promise_idx = crate::env::promise_batch_then(following, account_id);
                    lookup.insert(i, promise_idx);
                }
                PromiseQueueEvent::Action { index, action } => {
                    let promise_index = lookup[&index.0];
                    use PromiseAction::*;
                    match action {
                        CreateAccount => {
                            crate::env::promise_batch_action_create_account(promise_index)
                        }
                        DeployContract { code } => {
                            crate::env::promise_batch_action_deploy_contract(promise_index, code)
                        }
                        FunctionCall { method_name, arguments, amount, gas } => {
                            crate::env::promise_batch_action_function_call(
                                promise_index,
                                &method_name,
                                &arguments,
                                *amount,
                                gas.unwrap_or(function_call_gas),
                            )
                        }
                        Transfer { amount } => {
                            crate::env::promise_batch_action_transfer(promise_index, *amount)
                        }
                        Stake { amount, public_key } => crate::env::promise_batch_action_stake(
                            promise_index,
                            *amount,
                            public_key,
                        ),
                        AddFullAccessKey { public_key, nonce } => {
                            crate::env::promise_batch_action_add_key_with_full_access(
                                promise_index,
                                public_key,
                                *nonce,
                            )
                        }
                        AddAccessKey {
                            public_key,
                            allowance,
                            receiver_id,
                            method_names,
                            nonce,
                        } => crate::env::promise_batch_action_add_key_with_function_call(
                            promise_index,
                            public_key,
                            *nonce,
                            *allowance,
                            receiver_id,
                            method_names,
                        ),
                        DeleteKey { public_key } => {
                            crate::env::promise_batch_action_delete_key(promise_index, public_key)
                        }
                        DeleteAccount { beneficiary_id } => {
                            crate::env::promise_batch_action_delete_account(
                                promise_index,
                                beneficiary_id,
                            )
                        }
                    }
                }
                PromiseQueueEvent::Return { index } => {
                    let promise_index = lookup[&index.0];
                    crate::env::promise_return(promise_index);
                }
            }
        }

        queue.clear();
    });
}
