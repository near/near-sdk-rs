//! Bug class C4, storage accounting. On the mocked blockchain (the env host, which charges
//! storage exactly as the runtime does): for every call of a random history,
//! `Receipt::storage_delta()` of the call's logged `env::storage_*` calls equals the change of
//! `env::storage_usage()` across the call. The D5 guard: inside a method, a
//! `storage_usage()` call that does NOT panic means the flush will write nothing except the
//! root record (so a before/after measurement misses at most the root), and one that panics is
//! after a real change. The known gap (PROTOTYPE_NOTES, D5): a change to a root scalar alone
//! does not trip the guard; pinned.

use super::diff::{ops_gen, real_apply};
use super::fixture::MRoot;
use crate::state::model::{Model, Op};
use crate::state::testing::{self, OpKind};
use crate::state::{flush_root, load_root};
use crate::test_utils::VMContextBuilder;
use crate::{env, testing_env};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn quiet<R>(f: impl FnOnce() -> R) -> std::thread::Result<R> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let r = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(hook);
    r
}

/// Random engine, 500 histories of up to 60 operations on the env host (calls split at the
/// generator's call boundaries; views and retyped calls skipped): per call, logged
/// `storage_delta` == `storage_usage` after - before; after every operation inside a call, a
/// `storage_usage()` that does not panic implies the flush writes or removes only the root.
#[test]
fn c4_storage_delta_matches_storage_usage_and_the_guard_is_sound() {
    bolero::check!().with_generator(ops_gen()).with_iterations(500).for_each(|ops| {
        let _ = crate::mock::with_mocked_blockchain(|b| b.take_storage()); // fresh storage
        testing_env!(VMContextBuilder::new().build());
        testing::use_env_host();
        testing::env_log_start();
        let mut m = Model::default();
        let root_key = crate::state::model::root_key();
        let calls = ops.split(|o| matches!(o, Op::EndCall | Op::View(_) | Op::RecRetyped(..)));
        for call in calls {
            testing::begin_call();
            m.begin_call();
            let before = env::storage_usage();
            testing::env_log_take();
            let mut root = load_root::<MRoot>();
            let mut guard_quiet = true;
            for op in call {
                if matches!(op, Op::MidFlush) || m.would_panic(op) {
                    continue;
                }
                m.apply(op);
                real_apply(&mut root, op);
                if guard_quiet && quiet(env::storage_usage).is_err() {
                    guard_quiet = false;
                }
            }
            flush_root(&root);
            drop(root);
            let log = testing::env_log_take();
            testing::begin_call();
            let after = env::storage_usage();
            assert_eq!(log.storage_delta(), after as i64 - before as i64, "storage_delta disagrees with storage_usage");
            if guard_quiet {
                assert!(
                    log.ops.iter().filter(|o| matches!(o.kind, OpKind::Write | OpKind::Remove)).all(|o| o.key == root_key),
                    "the guard stayed quiet but the flush wrote below the root"
                );
            }
            let _ = m.end_call();
        }
        testing::env_log_stop();
    });
}

/// The guard trips inside a method after a collection write, and does not before it or after
/// the flush.
#[test]
fn c4_storage_usage_panics_inside_a_method_after_a_write() {
    testing_env!(VMContextBuilder::new().build());
    testing::use_env_host();
    testing::begin_call();
    let mut root = load_root::<MRoot>();
    let _ = env::storage_usage(); // before any write: fine
    root.flat.insert(1, vec![1]);
    let r = quiet(env::storage_usage);
    assert!(r.is_err(), "storage_usage() after a write inside a method did not panic");
    flush_root(&root);
    drop(root);
    let _ = env::storage_usage(); // after the flush: fine
}

/// KNOWN GAP, pinned: a method that changes only a root scalar passes the guard, yet its
/// flush writes the root record, so a before/after `storage_usage()` misses that delta.
#[test]
fn gap_root_scalar_only_change_does_not_trip_the_guard() {
    testing_env!(VMContextBuilder::new().build());
    testing::use_env_host();
    testing::begin_call();
    testing::env_log_start();
    let mut root = load_root::<MRoot>();
    let before = env::storage_usage();
    root.counter = 1;
    let during = env::storage_usage(); // GAP: no panic
    assert_eq!(during, before);
    flush_root(&root);
    drop(root);
    let log = testing::env_log_take();
    testing::env_log_stop();
    assert!(log.storage_delta() > 0, "the root write charged storage the method could not see");
}
