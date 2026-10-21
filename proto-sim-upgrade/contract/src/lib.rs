//! One small contract in five builds, for the storage v2 upgrade/migration search
//! (`../README.md`). Every mutating method bumps a root counter `seq` first and logs
//! `ev v=<version> seq=<n>`, so the harness can order the successful receipts of a case (a
//! deploy, a self-call landing later, two calls in one block) and know which code ran each.
//!
//! - `v1`: a v1 Borsh root, `store::IterableMap<String, u64>` under prefix `i`.
//! - `v2a`: the v2 root. `items` is `Migrating<IterableMap, Map>`, `groups` a `Map` of records
//!   that own a `Map` (for `remove_step`).
//! - `v2b`: v2a with the Group schema changed: tag 3 removed and `reserved`, tag 4 re-used for a
//!   different field on purpose (the hazard), `level` with `default`, `label` with `default_fn`,
//!   and a new `flags: Set` collection at tag 8.
//! - `v2c`: v2b plus `admin` at tag 9 marked `required` (the change the spec forbids).
//! - `v3`: `items` a bare v2 `Map` at the same tag (the wrapper off), Group as in v2b.

#[cfg(not(any(feature = "v1", feature = "v2a", feature = "v2b", feature = "v2c", feature = "v3")))]
compile_error!("pick one of v1, v2a, v2b, v2c, v3");

#[cfg(feature = "v1")]
pub const VERSION: &str = "v1";
#[cfg(feature = "v2a")]
pub const VERSION: &str = "v2a";
#[cfg(feature = "v2b")]
pub const VERSION: &str = "v2b";
#[cfg(feature = "v2c")]
pub const VERSION: &str = "v2c";
#[cfg(feature = "v3")]
pub const VERSION: &str = "v3";

#[cfg(feature = "v1")]
mod v1 {
    use near_sdk::store::IterableMap;
    use near_sdk::{PanicOnDefault, log, near};

    #[near(contract_state)]
    #[derive(PanicOnDefault)]
    pub struct App {
        items: IterableMap<String, u64>,
        seq: u64,
    }

    #[near]
    impl App {
        #[init]
        pub fn new() -> Self {
            Self { items: IterableMap::new(b"i"), seq: 0 }
        }
        pub fn version() -> String {
            crate::VERSION.into()
        }
        /// Inserts `s00..s{n-1}` with values 1000 + i.
        pub fn seed(&mut self, n: u32) {
            self.bump();
            for i in 0..n {
                self.items.insert(format!("s{i:02}"), 1000 + i as u64);
            }
        }
        pub fn insert(&mut self, k: String, v: u64) {
            self.bump();
            self.items.insert(k, v);
        }
        pub fn remove(&mut self, k: String) -> Option<u64> {
            self.bump();
            self.items.remove(&k)
        }
        pub fn get(&self, k: String) -> Option<u64> {
            self.items.get(&k).copied()
        }
        pub fn seq(&self) -> u64 {
            self.seq
        }
        pub fn old_len(&self) -> u32 {
            self.items.len()
        }
    }

    impl App {
        fn bump(&mut self) {
            self.seq += 1;
            log!("ev v={} seq={}", crate::VERSION, self.seq);
        }
    }
}

#[cfg(not(feature = "v1"))]
mod v2 {
    use core::num::NonZeroU32;
    #[cfg(not(feature = "v3"))]
    use near_sdk::state::Migrating;
    use near_sdk::serde_json::{Value, json};
    use near_sdk::state::Map;
    #[cfg(not(feature = "v2a"))]
    use near_sdk::state::Set;
    use near_sdk::store::IterableMap;
    use near_sdk::{Gas, NearToken, Promise, env, log, near};

    /// Gas a receipt keeps for itself before forwarding the rest to its continuation, and the
    /// least a continuation is sent with (`drain`, `clear_loop`). Below it the chain stops; the
    /// next call picks up where it left.
    pub const KEEP_TGAS: u64 = 10;
    pub const MIN_STEP_TGAS: u64 = 20;

    /// The gas left for a continuation, if enough.
    fn step_gas() -> Option<Gas> {
        let left = env::prepaid_gas()
            .saturating_sub(env::used_gas())
            .saturating_sub(Gas::from_tgas(KEEP_TGAS));
        (left >= Gas::from_tgas(MIN_STEP_TGAS)).then_some(left)
    }

    #[cfg(not(feature = "v3"))]
    pub type Items = Migrating<IterableMap<String, u64>, Map<String, u64>>;
    #[cfg(feature = "v3")]
    pub type Items = Map<String, u64>;

    /// The deployed v1 root, as its Borsh layout.
    #[near(serializers = [borsh])]
    pub struct V1Root {
        items: IterableMap<String, u64>,
        seq: u64,
    }

    #[cfg(not(feature = "v3"))]
    #[near(contract_state(layout = v2))]
    pub struct App {
        #[key(1, required)]
        items: Items,
        #[key(2)]
        groups: Map<String, Group>,
        #[key(3)]
        seq: u64,
    }

    /// The wrapper off: the same tag, now the bare v2 map (its entries are already there).
    #[cfg(feature = "v3")]
    #[near(contract_state(layout = v2))]
    pub struct App {
        #[key(1)]
        items: Items,
        #[key(2)]
        groups: Map<String, Group>,
        #[key(3)]
        seq: u64,
    }

    #[cfg(feature = "v2a")]
    #[near(storage)]
    pub struct Group {
        #[key(1)]
        owner: String,
        #[key(2)]
        weight: u64,
        #[key(3)]
        note: String,
        #[key(4)]
        locked: u64,
        #[key(5)]
        members: Map<String, u64>,
    }

    #[cfg(any(feature = "v2b", feature = "v3"))]
    #[near(storage, reserved(3))]
    pub struct Group {
        #[key(1)]
        owner: String,
        #[key(2)]
        weight: u64,
        /// HAZARD, on purpose: tag 4 was `locked: u64` in v2a. Same length, so old bytes decode.
        #[key(4)]
        claimable: u64,
        #[key(5)]
        members: Map<String, u64>,
        #[key(6, default = 7)]
        level: u32,
        #[key(7, default_fn = default_label)]
        label: String,
        #[key(8)]
        flags: Set<String>,
    }

    #[cfg(feature = "v2c")]
    #[near(storage, reserved(3))]
    pub struct Group {
        #[key(1)]
        owner: String,
        #[key(2)]
        weight: u64,
        #[key(4)]
        claimable: u64,
        #[key(5)]
        members: Map<String, u64>,
        #[key(6, default = 7)]
        level: u32,
        #[key(7, default_fn = default_label)]
        label: String,
        #[key(8)]
        flags: Set<String>,
        /// Added `required` to records that already exist: forbidden by the spec.
        #[key(9, required)]
        admin: String,
    }

    #[cfg(not(feature = "v2a"))]
    fn default_label() -> String {
        "unlabeled".into()
    }

    fn no_group() -> ! {
        env::panic_str("no such group")
    }

    #[near]
    impl App {
        pub fn version() -> String {
            crate::VERSION.into()
        }

        /// From the v1 root, in the deploy transaction.
        #[cfg(not(feature = "v3"))]
        #[private]
        #[init]
        pub fn migrate() -> Self {
            let old: V1Root = env::state_read().unwrap_or_else(|| env::panic_str("no v1 STATE"));
            env::storage_remove(b"STATE");
            let seq = old.seq + 1;
            log!("ev v={} seq={}", crate::VERSION, seq);
            Self { items: Migrating::new(old.items), groups: Default::default(), seq }
        }

        // ---- items ----

        pub fn insert(&mut self, k: String, v: u64) {
            self.bump();
            #[cfg(not(feature = "v3"))]
            self.items.insert(k, v);
            #[cfg(feature = "v3")]
            let _ = self.items.insert(k, v);
        }
        pub fn remove(&mut self, k: String) -> Option<u64> {
            self.bump();
            self.items.remove(&k)
        }
        pub fn get(&self, k: String) -> Option<u64> {
            #[cfg(not(feature = "v3"))]
            return self.items.get(&k);
            #[cfg(feature = "v3")]
            return self.items.get(&k).copied();
        }

        #[cfg(not(feature = "v3"))]
        pub fn migrate_batch(&mut self, n: u32) -> u32 {
            self.bump();
            self.items.migrate_batch(n)
        }
        /// `migrate_batch(n)`, then a self-call for the next batch while the old side has
        /// entries and `rounds` allow.
        #[cfg(not(feature = "v3"))]
        #[private]
        pub fn drain(&mut self, n: u32, rounds: u32) -> u32 {
            self.bump();
            let moved = self.items.migrate_batch(n);
            if rounds > 1 && !self.items.old_is_empty() {
                if let Some(gas) = step_gas() {
                    Promise::new(env::current_account_id())
                        .function_call(
                            "drain",
                            json!({ "n": n, "rounds": rounds - 1 }).to_string().into_bytes(),
                            NearToken::from_yoctonear(0),
                            gas,
                        )
                        .detach();
                }
            }
            moved
        }
        #[cfg(not(feature = "v3"))]
        pub fn old_len(&self) -> u32 {
            self.items.old_side().len()
        }
        #[cfg(not(feature = "v3"))]
        pub fn old_is_empty(&self) -> bool {
            self.items.old_is_empty()
        }
        pub fn new_len(&self) -> u32 {
            #[cfg(not(feature = "v3"))]
            return self.items.new_side().len();
            #[cfg(feature = "v3")]
            return self.items.len();
        }

        // ---- groups ----

        pub fn create_group(&mut self, g: String, weight: u64, x: u64) {
            self.bump();
            #[cfg(feature = "v2a")]
            let grp = Group {
                owner: "o".into(),
                weight,
                note: format!("n{x}"),
                locked: x,
                members: Default::default(),
            };
            #[cfg(any(feature = "v2b", feature = "v3"))]
            let grp = Group {
                owner: "o".into(),
                weight,
                claimable: 0,
                members: Default::default(),
                level: x as u32,
                label: format!("l{x}"),
                flags: Default::default(),
            };
            #[cfg(feature = "v2c")]
            let grp = Group {
                owner: "o".into(),
                weight,
                claimable: 0,
                members: Default::default(),
                level: x as u32,
                label: format!("l{x}"),
                flags: Default::default(),
                admin: "a".into(),
            };
            if self.groups.try_insert(g, grp).is_err() {
                env::panic_str("group exists");
            }
        }
        pub fn set_weight(&mut self, g: String, w: u64) {
            self.bump();
            self.groups.get_mut(&g).unwrap_or_else(|| no_group()).weight = w;
        }
        /// v2a: `locked`; v2b and later: `claimable` (the re-used tag).
        pub fn set_field4(&mut self, g: String, x: u64) {
            self.bump();
            let grp = self.groups.get_mut(&g).unwrap_or_else(|| no_group());
            #[cfg(feature = "v2a")]
            {
                grp.locked = x;
            }
            #[cfg(not(feature = "v2a"))]
            {
                grp.claimable = x;
            }
        }
        #[cfg(not(feature = "v2a"))]
        pub fn set_level(&mut self, g: String, x: u64) {
            self.bump();
            self.groups.get_mut(&g).unwrap_or_else(|| no_group()).level = x as u32;
        }
        #[cfg(not(feature = "v2a"))]
        pub fn set_label(&mut self, g: String, x: u64) {
            self.bump();
            self.groups.get_mut(&g).unwrap_or_else(|| no_group()).label = format!("L{x}");
        }
        pub fn add_member(&mut self, g: String, m: String, v: u64) {
            self.bump();
            let _ = self.groups.at_mut(&g).unwrap_or_else(|| no_group()).members.insert(m, v);
        }
        pub fn remove_member(&mut self, g: String, m: String) {
            self.bump();
            let _ = self.groups.at_mut(&g).unwrap_or_else(|| no_group()).members.remove(&m);
        }
        #[cfg(not(feature = "v2a"))]
        pub fn add_flag(&mut self, g: String, f: String) {
            self.bump();
            let _ = self.groups.at_mut(&g).unwrap_or_else(|| no_group()).flags.insert(f);
        }
        /// One bounded clearing step. Returns true once the group is gone (or was absent).
        pub fn remove_step(&mut self, g: String, n: u32) -> bool {
            self.bump();
            let n = NonZeroU32::new(n).unwrap_or_else(|| env::panic_str("n must be > 0"));
            self.groups.remove_step(&g, n).is_done()
        }
        /// `remove_step(g, n)`, then a self-call for the next step while it is pending and
        /// `rounds` allow.
        #[private]
        pub fn clear_loop(&mut self, g: String, n: u32, rounds: u32) -> bool {
            let done = self.remove_step(g.clone(), n);
            if !done && rounds > 1 {
                if let Some(gas) = step_gas() {
                    Promise::new(env::current_account_id())
                        .function_call(
                            "clear_loop",
                            json!({ "g": g, "n": n, "rounds": rounds - 1 }).to_string().into_bytes(),
                            NearToken::from_yoctonear(0),
                            gas,
                        )
                        .detach();
                }
            }
            done
        }
        /// Unbounded removal (allowed on a clearing record).
        pub fn remove_group(&mut self, g: String) -> bool {
            self.bump();
            self.groups.remove(&g).is_some()
        }

        // ---- views ----

        pub fn seq(&self) -> u64 {
            self.seq
        }
        pub fn group(&self, g: String) -> Option<Value> {
            let grp = self.groups.get(&g)?;
            #[cfg(feature = "v2a")]
            return Some(json!({
                "owner": grp.owner, "weight": grp.weight, "note": grp.note, "locked": grp.locked,
            }));
            #[cfg(any(feature = "v2b", feature = "v3"))]
            return Some(json!({
                "owner": grp.owner, "weight": grp.weight, "claimable": grp.claimable,
                "level": grp.level, "label": grp.label,
            }));
            #[cfg(feature = "v2c")]
            return Some(json!({
                "owner": grp.owner, "weight": grp.weight, "claimable": grp.claimable,
                "level": grp.level, "label": grp.label, "admin": grp.admin,
            }));
        }
        pub fn members_len(&self, g: String) -> u32 {
            self.groups.at(&g).members.len()
        }
        pub fn member(&self, g: String, m: String) -> Option<u64> {
            self.groups.at(&g).members.get(&m).copied()
        }
        pub fn groups_len(&self) -> u32 {
            self.groups.len()
        }
        pub fn storage_usage() -> u64 {
            env::storage_usage()
        }
    }

    impl App {
        fn bump(&mut self) {
            self.seq += 1;
            log!("ev v={} seq={}", crate::VERSION, self.seq);
        }
    }
}
