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
    use near_sdk::state::{LookupMap, Map};
    use near_sdk::{AccountId, env, near};

    #[near(contract_state(layout = v2))]
    pub struct Bench {
        #[key(1)] owner: Option<AccountId>,
        #[key(2)] fee_bps: u16,
        #[key(3)] map: Map<AccountId, u128>,
        #[key(4)] lookup: LookupMap<AccountId, u128>,
        #[key(5)] blobs: Map<AccountId, Vec<u8>>,
        #[key(6)] accounts: Map<AccountId, Account>,
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
