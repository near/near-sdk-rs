use std::cell::RefCell;

use super::PromiseAction;
use crate::AccountId;

thread_local! {
    static QUEUED_PROMISES: RefCell<Vec<PromiseQueueEvent>> = RefCell::new(Vec::new());
}

#[derive(Clone, Copy, Debug)]
pub struct QueueIndex(usize);

#[non_exhaustive]
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

/// Schedules queued function calls, which will split remaining gas
pub fn schedule_queued_promises() {
    QUEUED_PROMISES.with(|q| {
        let mut queue = q.borrow_mut();
        // TODO get how much gas is remaining
        // TODO iterate over to determine static gas defined and number of unspecified
        // for QueuedFunctionCall { promise_index, method_name, arguments, amount, gas } in
        //     queue.iter()
        // {
        //     if gas.is_none() {
        //         panic!("FIRING: {}", method_name);
        //     }
        //     promise_batch_action_function_call(
        //         promise_index,
        //         &method_name,
        //         &arguments,
        //         amount,
        //         // TODO switch this from unwrap, this value should be calculated based on above
        //         gas.unwrap(),
        //     )
        // }
        queue.clear();
    });
}
