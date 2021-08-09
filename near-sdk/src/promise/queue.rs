use std::cell::RefCell;

use super::PromiseAction;
use crate::{env, AccountId, Gas, PromiseIndex};

thread_local! {
    static QUEUED_PROMISES: RefCell<Vec<PromiseQueueEvent>> = RefCell::new(Vec::new());
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueueIndex(usize);

#[non_exhaustive]
pub enum PromiseQueueEvent {
    CreateBatch { account_id: AccountId },
    BatchAnd { promise_a: QueueIndex, promise_b: QueueIndex },
    BatchThen { previous_index: QueueIndex, account_id: AccountId },
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

pub(super) fn upgrade_to_then(previous_index: QueueIndex, next: QueueIndex) {
    QUEUED_PROMISES.with(|q| {
        let mut queue = q.borrow_mut();
        let event_mut = queue.get_mut(next.0).unwrap_or_else(|| unreachable!());

        // Replace current event with low cost variant. It gets replaced at the end of the function
        // so this doesn't matter. This is to avoid a clone of the account_id.
        let event =
            core::mem::replace(event_mut, PromiseQueueEvent::Return { index: QueueIndex(0) });
        let account_id = if let PromiseQueueEvent::CreateBatch { account_id } = event {
            account_id
        } else {
            // This is unreachable because `then` can only be called on a promise once
            // and it is always in `CreateBatch` state until then.
            unreachable!()
        };

        *event_mut = PromiseQueueEvent::BatchThen { previous_index, account_id };
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
        if let PromiseQueueEvent::Action { action: PromiseAction::FunctionCall { .. }, .. } = event
        {
            count_remaining += 1;
            // if let Some(specified) = gas {
            //     // Gas was specified, remove this from pool of funds
            //     remaining_gas -= *specified;
            // } else {
            //     count_remaining += 1;
            // }
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

        let mut lookup = vec![PromiseIndex::MAX; queue.len()];

        for (i, event) in queue.iter().enumerate() {
            match event {
                PromiseQueueEvent::CreateBatch { account_id } => {
                    let promise_idx = crate::env::promise_batch_create(account_id);
                    lookup.insert(i, promise_idx);
                }
                PromiseQueueEvent::BatchAnd { promise_a, promise_b } => {
                    //* indices guaranteed to be in lookup as the queue index would be used
                    // 	to create the `and` promise.
                    let a = lookup[promise_a.0];
                    let b = lookup[promise_b.0];
                    let promise_idx = crate::env::promise_and(&[a, b]);
                    lookup[i] = promise_idx;
                }
                PromiseQueueEvent::BatchThen { previous_index, account_id } => {
                    let index = lookup[previous_index.0];
                    let promise_idx = crate::env::promise_batch_then(index, account_id);
                    lookup[i] = promise_idx;
                }
                PromiseQueueEvent::Action { index, action } => {
                    let promise_index = lookup[index.0];
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
                                method_name,
                                arguments,
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
                    let promise_index = lookup[index.0];
                    crate::env::promise_return(promise_index);
                }
            }
        }

        // TODO should this actually be cleared? Is there any benefit
        queue.clear();
    });
}
