//! Gas-measurement contract. The same methods over v2 (`near_sdk::state`) or v1
//! (`near_sdk::store`) collections, so the simulator can compare them call for call.
//!
//! Map keys are `AccountId`s, values `u128` (the doc's `Map<AccountId, u128>` shape).
//! `blobs` holds byte vectors for the removal-beside-a-large-neighbor test. `accounts` is the
//! spec's `Exchange.accounts` shape, for `balance_of` (v2 only).

#[cfg(all(feature = "v1", feature = "v2"))]
compile_error!("pick one of v1, v2");

use near_sdk::env;

/// Deterministic seeded ids: `hex(sha256("u{i}"))[..16] ++ ".near"`, 21 bytes.
pub fn gen_id(i: u32) -> near_sdk::AccountId {
    let h = env::sha256(format!("u{i}").as_bytes());
    let mut s = String::with_capacity(21);
    for b in &h[..8] {
        s.push_str(&format!("{b:02x}"));
    }
    s.push_str(".near");
    s.parse().unwrap()
}

#[cfg(feature = "v2")]
mod v2 {
    use near_sdk::json_types::U128;
    #[cfg(feature = "ext")]
    use core::num::NonZeroU32;
    #[cfg(feature = "ext")]
    use near_sdk::json_types::U64;
    use near_sdk::state::{LookupMap, Map};
    #[cfg(feature = "ext")]
    use near_sdk::state::Page;
    use near_sdk::{AccountId, env, near};

    #[cfg(not(feature = "ext"))]
    #[near(contract_state(layout = v2))]
    pub struct Bench {
        #[key(1)] owner: Option<AccountId>,
        #[key(2)] fee_bps: u16,
        #[key(3)] map: Map<AccountId, u128>,
        #[key(4)] lookup: LookupMap<AccountId, u128>,
        #[key(5)] blobs: Map<AccountId, Vec<u8>>,
        #[key(6)] accounts: Map<AccountId, Account>,
    }

    /// The `ext` build: the same root plus a `LookupMap` of collection-owning records, for the
    /// `LookupMap::try_insert` row (D9 flag-key existence check on a miss).
    #[cfg(feature = "ext")]
    #[near(contract_state(layout = v2))]
    pub struct Bench {
        #[key(1)] owner: Option<AccountId>,
        #[key(2)] fee_bps: u16,
        #[key(3)] map: Map<AccountId, u128>,
        #[key(4)] lookup: LookupMap<AccountId, u128>,
        #[key(5)] blobs: Map<AccountId, Vec<u8>>,
        #[key(6)] accounts: Map<AccountId, Account>,
        #[key(7)] laccounts: LookupMap<AccountId, Account>,
    }

    #[near(storage)]
    pub struct Account {
        #[key(1)] nonce: u64,
        #[key(2)] balances: Map<String, u128>,
    }

    #[near]
    impl Bench {
        /// No `self`: no root read, no flush. The floor of a function call.
        pub fn ping() {}
        /// Root read and decode only.
        pub fn noop_view(&self) {}
        /// Root read, decode and an (empty) flush.
        pub fn noop_mut(&mut self) {}

        pub fn set_owner(&mut self) {
            self.owner = Some(env::predecessor_account_id());
            self.fee_bps = 30;
        }

        pub fn map_insert(&mut self, id: AccountId, v: U128) {
            let _ = self.map.insert(id, v.0);
        }
        pub fn map_remove(&mut self, id: AccountId) {
            let _ = self.map.remove(&id);
        }
        pub fn lookup_insert(&mut self, id: AccountId, v: U128) {
            self.lookup.insert(id, v.0);
        }
        pub fn lookup_remove(&mut self, id: AccountId) {
            let _ = self.lookup.remove(&id);
        }
        pub fn blob_insert(&mut self, id: AccountId, size: u32) {
            let _ = self.blobs.insert(id, vec![7u8; size as usize]);
        }
        pub fn blob_remove(&mut self, id: AccountId) {
            let _ = self.blobs.remove(&id);
        }

        /// Seeds `n` generated keys from `start` into `map` (and `lookup` if `both`).
        pub fn seed(&mut self, start: u32, n: u32, which: String) {
            for i in start..start + n {
                let id = crate::gen_id(i);
                match which.as_str() {
                    "map" => {
                        self.map.insert(id, i as u128);
                    }
                    "lookup" => self.lookup.insert(id, i as u128),
                    #[cfg(feature = "ext")]
                    "accounts" => {
                        let _ = self.accounts.try_insert(id, Account { nonce: i as u64, ..Default::default() });
                    }
                    _ => env::panic_str("which"),
                }
            }
        }

        pub fn map_len(&self) -> u32 {
            self.map.len()
        }
        pub fn map_get(&self, id: AccountId) -> Option<U128> {
            self.map.get(&id).map(|v| U128(*v))
        }
        pub fn lookup_get(&self, id: AccountId) -> Option<U128> {
            self.lookup.get(&id).map(|v| U128(*v))
        }

        // The spec's Exchange methods.
        pub fn deposit(&mut self, token: String, amount: U128) {
            let acct = self.accounts.entry(env::predecessor_account_id()).or_default();
            *acct.balances.entry(token).or_default() += amount.0;
        }
        pub fn balance_of(&self, id: AccountId, token: String) -> U128 {
            self.accounts.at(&id).balances.get(&token).copied().unwrap_or(0).into()
        }
    }

    /// Methods for the rows added on Oct 18 (`ext` build only, so the Oct 17 rows run on a wasm
    /// with the Oct 17 method set).
    #[cfg(feature = "ext")]
    #[near]
    impl Bench {
        /// Cursor page of `map` (API round 3: `page` carries `revision`). `page_with` maps the
        /// `u128` values to JSON strings; same reads as `page`.
        pub fn map_page(&self, after: Option<AccountId>, limit: u32) -> Page<AccountId, U128> {
            self.map.page_with(after.as_ref(), limit, |_, v| U128(*v)).expect("cursor gone")
        }

        // Collection-owning records (`Account` holds a `Map`).
        /// `LookupMap::try_insert` of a collection-owning record: 2 existence checks (record
        /// key, then its flag key) + 1 write.
        pub fn laccount_try_insert(&mut self, id: AccountId, nonce: u64) {
            self.laccounts.try_insert(id, Account { nonce, ..Default::default() }).expect("account exists");
        }
        /// `Map::try_insert` of a new record that owns a collection (API round 3, was
        /// `insert_new`): the index read decides, no flag-key check.
        pub fn account_try_insert(&mut self, id: AccountId, nonce: u64) {
            self.accounts.try_insert(id, Account { nonce, ..Default::default() }).expect("account exists");
        }
        /// Creates (or opens) `id` and writes `n` balances `t0..t{n-1}`.
        pub fn account_fill(&mut self, id: AccountId, n: u32) {
            let a = self.accounts.entry(id).or_default();
            for i in 0..n {
                let _ = a.balances.insert(format!("t{i}"), i as u128 + 1);
            }
        }
        /// One bounded removal step (API round 3/5: `remove_step`, budget counts host removes,
        /// minimum 3). Returns `true` once the record is gone.
        pub fn account_remove_step(&mut self, id: AccountId, n: u32) -> bool {
            self.accounts.remove_step(&id, NonZeroU32::new(n).expect("n > 0")).is_done()
        }
        pub fn accounts_len(&self) -> u32 {
            self.accounts.len()
        }
        pub fn account_nonce(&self, id: AccountId) -> Option<U64> {
            self.accounts.get(&id).map(|a| U64(a.nonce))
        }
    }
}

#[cfg(feature = "v1")]
mod v1 {
    use near_sdk::json_types::U128;
    use near_sdk::store::{IterableMap, LookupMap};
    use near_sdk::{AccountId, PanicOnDefault, env, near};

    #[near(contract_state)]
    #[derive(PanicOnDefault)]
    pub struct Bench {
        owner: Option<AccountId>,
        fee_bps: u16,
        map: IterableMap<AccountId, u128>,
        lookup: LookupMap<AccountId, u128>,
        blobs: IterableMap<AccountId, Vec<u8>>,
    }

    #[near]
    impl Bench {
        #[init]
        pub fn new() -> Self {
            Self {
                owner: Some(env::predecessor_account_id()),
                fee_bps: 30,
                map: IterableMap::new(b"m"),
                lookup: LookupMap::new(b"l"),
                blobs: IterableMap::new(b"b"),
            }
        }

        pub fn ping() {}
        pub fn noop_view(&self) {}
        pub fn noop_mut(&mut self) {}

        pub fn map_insert(&mut self, id: AccountId, v: U128) {
            let _ = self.map.insert(id, v.0);
        }
        pub fn map_remove(&mut self, id: AccountId) {
            let _ = self.map.remove(&id);
        }
        pub fn lookup_insert(&mut self, id: AccountId, v: U128) {
            let _ = self.lookup.insert(id, v.0);
        }
        pub fn lookup_remove(&mut self, id: AccountId) {
            let _ = self.lookup.remove(&id);
        }
        pub fn blob_insert(&mut self, id: AccountId, size: u32) {
            let _ = self.blobs.insert(id, vec![7u8; size as usize]);
        }
        pub fn blob_remove(&mut self, id: AccountId) {
            let _ = self.blobs.remove(&id);
        }

        pub fn seed(&mut self, start: u32, n: u32, which: String) {
            for i in start..start + n {
                let id = crate::gen_id(i);
                match which.as_str() {
                    "map" => {
                        self.map.insert(id, i as u128);
                    }
                    "lookup" => {
                        self.lookup.insert(id, i as u128);
                    }
                    _ => env::panic_str("which"),
                }
            }
        }

        pub fn map_len(&self) -> u32 {
            self.map.len()
        }
        pub fn map_get(&self, id: AccountId) -> Option<U128> {
            self.map.get(&id).map(|v| U128(*v))
        }
        pub fn lookup_get(&self, id: AccountId) -> Option<U128> {
            self.lookup.get(&id).map(|v| U128(*v))
        }
    }
}
