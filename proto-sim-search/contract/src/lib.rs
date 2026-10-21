//! Storage v2 search contract. One v2 root that covers every shape the reference model
//! (`near-sdk/src/state/model.rs`) drives, written with the real derive, so the simulator
//! search runs the real `#[near]` wrapper, the generated `clear_children_some`, the `at` /
//! `at_mut` views and the JSON round trip of `Page` / `KeyPage` on nearcore's runtime.
//!
//! Every mutating method takes `mode`:
//! - 0: plain;
//! - 1: panic at the end of the body, before the wrapper's flush (G4: nothing written);
//! - 2: `state_write()` by hand, then panic (F3: the runtime reverts a flushed receipt);
//! - 3: `state_write()` by hand, then bump the root counter by 1 and return (the model's
//!   `MidFlush`: a root change after a mid-call flush must still land in the final flush).
//!
//! `acct_remove_chain` drives `remove_step` through self-calls until `Done`, so the search has a
//! spawned receipt to starve (a forced `OutOfGas` cannot land on a transaction's first receipt).
//!
//! `probe_has` / `probe_read` take no `self`: they call the host directly, bypassing the v2
//! cache, so the harness can look for orphan keys under a removed path.

use core::num::NonZeroU32;
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::Serialize;
use near_sdk::state::{ContractState, Lazy, LookupMap, Map, Set, Vector};
use near_sdk::{AccountId, Gas, GasWeight, NearToken, Promise, PromiseOrValue, env, near};

#[near(contract_state(layout = v2))]
pub struct Store {
    #[key(1)]
    counter: u64,
    #[key(2)]
    accounts: Map<AccountId, Account>,
    #[key(3)]
    lookup: LookupMap<AccountId, u64>,
    #[key(4)]
    members: Set<AccountId>,
}

#[near(storage)]
pub struct Account {
    #[key(1)]
    nonce: u64,
    #[key(2)]
    balances: Map<String, u64>,
    #[key(3)]
    log: Vector<u64>,
    #[key(4)]
    blob: Lazy<Vec<u8>>,
}

/// What `acct_get` returns: the record's scalar through `get` (which reads a record being
/// cleared at its flag key) and its collections through `at`.
#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AcctView {
    pub nonce: u64,
    pub balances_len: u32,
    pub balances: Vec<(String, u64)>,
    pub log_len: u32,
    pub log: Vec<u64>,
    pub blob_len: Option<u32>,
}

/// `Page` with `CursorGone` folded in, so a gone cursor is a value, not a failed receipt.
#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PageOut<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
    pub revision: u64,
    pub gone: bool,
}

/// The blob `blob_set(size)` writes: deterministic, so the harness can predict its bytes.
pub fn blob_of(size: u32) -> Vec<u8> {
    vec![(size % 251) as u8; size as usize]
}

fn step_n(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap_or_else(|| env::panic_str("remove_step: n must be > 0"))
}

#[near]
impl Store {
    fn fin(&mut self, mode: u8) {
        match mode {
            0 => {}
            1 => env::panic_str("planned panic"),
            2 => {
                self.state_write();
                env::panic_str("planned panic after flush");
            }
            3 => {
                self.state_write();
                self.counter = self.counter.wrapping_add(1);
            }
            _ => env::panic_str("bad mode"),
        }
    }

    // ---- root scalar ----
    pub fn bump(&mut self, by: u64, mode: u8) -> u64 {
        self.counter = self.counter.wrapping_add(by);
        let r = self.counter;
        self.fin(mode);
        r
    }

    // ---- Map<AccountId, Account> ----
    pub fn acct_try_insert(&mut self, who: AccountId, nonce: u64, mode: u8) -> bool {
        let r = self.accounts.try_insert(who, Account { nonce, ..Default::default() }).is_ok();
        self.fin(mode);
        r
    }
    /// `entry(k).or_default()`; returns the nonce.
    pub fn acct_touch(&mut self, who: AccountId, mode: u8) -> u64 {
        let r = self.accounts.entry(who).or_default().nonce;
        self.fin(mode);
        r
    }
    /// `get_mut`; false if absent.
    pub fn acct_set_nonce(&mut self, who: AccountId, nonce: u64, mode: u8) -> bool {
        let r = match self.accounts.get_mut(&who) {
            Some(a) => {
                a.nonce = nonce;
                true
            }
            None => false,
        };
        self.fin(mode);
        r
    }
    /// Unbounded `remove`; returns the removed record's nonce.
    pub fn acct_remove(&mut self, who: AccountId, mode: u8) -> Option<u64> {
        let r = self.accounts.remove(&who).map(|a| a.nonce);
        self.fin(mode);
        r
    }
    pub fn acct_discard(&mut self, who: AccountId, mode: u8) -> bool {
        let r = self.accounts.discard(&who);
        self.fin(mode);
        r
    }
    /// One bounded step; true once the record is gone.
    pub fn acct_remove_step(&mut self, who: AccountId, n: u32, mode: u8) -> bool {
        let r = self.accounts.remove_step(&who, step_n(n)).is_done();
        self.fin(mode);
        r
    }
    /// `remove_step` until `Done`, one self-call per step; the chain's last link returns true.
    pub fn acct_remove_chain(&mut self, who: AccountId, n: u32) -> PromiseOrValue<bool> {
        if self.accounts.remove_step(&who, step_n(n)).is_done() {
            return PromiseOrValue::Value(true);
        }
        let args = near_sdk::serde_json::json!({ "who": who, "n": n }).to_string();
        PromiseOrValue::Promise(Promise::new(env::current_account_id()).function_call_weight(
            "acct_remove_chain",
            args.into_bytes(),
            NearToken::from_yoctonear(0),
            Gas::from_tgas(3),
            GasWeight(1),
        ))
    }
    /// Creates (or opens) `who` and fills `n` balances `t0..`, `n` log entries and a blob.
    pub fn acct_bulk(&mut self, who: AccountId, n: u32, blob: u32, mode: u8) -> u64 {
        let a = self.accounts.entry(who).or_default();
        for i in 0..n {
            let _ = a.balances.insert(format!("t{i}"), i as u64 + 1);
            a.log.push(i as u64 + 100);
        }
        a.blob.set(blob_of(blob));
        let r = a.nonce;
        self.fin(mode);
        r
    }

    // ---- below a record, through at_mut ----
    /// None if the record is absent; else whether the token was new.
    pub fn bal_set(&mut self, who: AccountId, token: String, amount: u64, mode: u8) -> Option<bool> {
        let r = self.accounts.at_mut(&who).map(|a| a.balances.insert(token, amount));
        self.fin(mode);
        r
    }
    pub fn bal_remove(&mut self, who: AccountId, token: String, mode: u8) -> Option<u64> {
        let r = self.accounts.at_mut(&who).and_then(|a| a.balances.remove(&token));
        self.fin(mode);
        r
    }
    pub fn log_push(&mut self, who: AccountId, v: u64, mode: u8) -> bool {
        let r = match self.accounts.at_mut(&who) {
            Some(a) => {
                a.log.push(v);
                true
            }
            None => false,
        };
        self.fin(mode);
        r
    }
    pub fn log_pop(&mut self, who: AccountId, mode: u8) -> Option<u64> {
        let r = self.accounts.at_mut(&who).and_then(|a| a.log.pop());
        self.fin(mode);
        r
    }
    pub fn blob_set(&mut self, who: AccountId, size: u32, mode: u8) -> bool {
        let r = match self.accounts.at_mut(&who) {
            Some(a) => {
                a.blob.set(blob_of(size));
                true
            }
            None => false,
        };
        self.fin(mode);
        r
    }
    pub fn blob_remove(&mut self, who: AccountId, mode: u8) -> bool {
        let r = match self.accounts.at_mut(&who) {
            Some(a) => {
                a.blob.remove();
                true
            }
            None => false,
        };
        self.fin(mode);
        r
    }

    // ---- LookupMap ----
    pub fn look_insert(&mut self, who: AccountId, v: u64, mode: u8) {
        self.lookup.insert(who, v);
        self.fin(mode);
    }
    pub fn look_try_insert(&mut self, who: AccountId, v: u64, mode: u8) -> bool {
        let r = self.lookup.try_insert(who, v).is_ok();
        self.fin(mode);
        r
    }
    pub fn look_remove(&mut self, who: AccountId, mode: u8) -> Option<u64> {
        let r = self.lookup.remove(&who);
        self.fin(mode);
        r
    }

    // ---- Set ----
    pub fn mem_insert(&mut self, who: AccountId, mode: u8) -> bool {
        let r = self.members.insert(who);
        self.fin(mode);
        r
    }
    pub fn mem_remove(&mut self, who: AccountId, mode: u8) -> bool {
        let r = self.members.remove(&who);
        self.fin(mode);
        r
    }

    // ---- reads (views, also called as transactions so they sit in the plan's order) ----
    pub fn counter(&self) -> u64 {
        self.counter
    }
    pub fn accounts_len(&self) -> u32 {
        self.accounts.len()
    }
    pub fn acct_contains(&self, who: AccountId) -> bool {
        self.accounts.contains_key(&who)
    }
    pub fn acct_get(&self, who: AccountId) -> Option<AcctView> {
        let nonce = self.accounts.get(&who)?.nonce;
        let at = self.accounts.at(&who);
        Some(AcctView {
            nonce,
            balances_len: at.balances.len(),
            balances: at.balances.iter().map(|(k, v)| (k, *v)).collect(),
            log_len: at.log.len(),
            log: at.log.iter().copied().collect(),
            blob_len: at.blob.get().map(|b| b.len() as u32),
        })
    }
    pub fn look_get(&self, who: AccountId) -> Option<u64> {
        self.lookup.get(&who).copied()
    }
    pub fn mem_contains(&self, who: AccountId) -> bool {
        self.members.contains(&who)
    }
    /// Cursor page of `accounts`, values = nonce.
    pub fn accounts_page(&self, after: Option<AccountId>, limit: u32) -> PageOut<(String, u64)> {
        match self.accounts.page_with(after.as_ref(), limit, |_, a| a.nonce) {
            Ok(p) => PageOut {
                items: p.items.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
                next: p.next.map(|k| k.to_string()),
                revision: p.revision,
                gone: false,
            },
            Err(_) => PageOut { items: vec![], next: None, revision: 0, gone: true },
        }
    }
    /// `KeyPage` of `members`.
    pub fn members_page(&self, after: Option<AccountId>, limit: u32) -> PageOut<String> {
        match self.members.page(after.as_ref(), limit) {
            Ok(p) => PageOut {
                items: p.keys.into_iter().map(|k| k.to_string()).collect(),
                next: p.next.map(|k| k.to_string()),
                revision: p.revision,
                gone: false,
            },
            Err(_) => PageOut { items: vec![], next: None, revision: 0, gone: true },
        }
    }
    /// `Page` of a record's nested `balances` (through `at`, no record read).
    pub fn balances_page(&self, who: AccountId, after: Option<String>, limit: u32) -> PageOut<(String, u64)> {
        match self.accounts.at(&who).balances.page(after.as_ref(), limit) {
            Ok(p) => PageOut { items: p.items, next: p.next, revision: p.revision, gone: false },
            Err(_) => PageOut { items: vec![], next: None, revision: 0, gone: true },
        }
    }

    // ---- raw host probes (no self, no cache) ----
    pub fn probe_has(keys: Vec<Base64VecU8>) -> Vec<bool> {
        keys.iter().map(|k| env::storage_has_key(&k.0)).collect()
    }
    pub fn probe_read(key: Base64VecU8) -> Option<Base64VecU8> {
        env::storage_read(&key.0).map(Base64VecU8)
    }
    pub fn storage_usage() -> u64 {
        env::storage_usage()
    }
}
