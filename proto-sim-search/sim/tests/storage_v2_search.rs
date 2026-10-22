//! Storage v2's invariants as a stateful property search on near-contract-sim: nearcore's
//! runtime in-process, the real trie, real storage accounting, real gas.
//!
//! The contract (`../contract`, built to `../res/store.wasm` by `../build.sh`) is one v2 root:
//! a root scalar, `Map<AccountId, Account>` where `Account` owns a nested `Map`, a `Vector` and
//! a `Lazy<Vec<u8>>`, a `LookupMap` and a `Set`. Its methods are the reference model's op set
//! (`near-sdk/src/state/model.rs`): insert / overwrite / remove at the root and below a record,
//! `try_insert`, `entry().or_default()`, `remove_step(k, n)` with any `n`, cursor pages, plus
//! failure modes (panic before the flush, panic after a hand flush, a hand flush mid-call).
//!
//! What is judged (SDK properties only; ported to near-contract-sim main @ 8e8642db, Oct 20):
//! - the mirror, at rest: every receipt the contract ran is replayed through [`Model`] in
//!   execution order (`Sim::receipts_in`); each return value must equal the model's, a panic must
//!   be one the model predicts, and a receipt that failed (any reason) is dropped from the model;
//! - exact state, at rest: the contract's whole trie (`Sim::view_state`) must equal the model's
//!   concretization, key for key and byte for byte, built with the spec's key grammar. This is
//!   G2/C2 (no key under a removed path), C4's bytes, and F3/G4 as far as the SDK owns it (a
//!   failed receipt's writes reaching the trie would be an extra key);
//! - C5: a record mid `remove_step` has no record key, its bytes verbatim at the flag key,
//!   only keys it held when clearing began, and stays readable; every `remove_step` receipt
//!   issues at most `n` host removes (the real gas profile's count); `Done` leaves nothing; per
//!   op, a `remove_step` never adds a key under its record (not judged when ops overlap);
//! - C6: every page equals the model's key-vector slice, or is `CursorGone` exactly when the
//!   cursor key is absent; a walk at one revision returns no key twice;
//! - the raw probe: `storage_has_key` through the contract (no cache) on the record, flag and
//!   child keys of every absent record is false.
//!
//! Runtime sanity, not an SDK property (labelled `runtime:`): `storage_usage` equals the empty
//! contract's plus key + value + 40 per key (nearcore's accounting rule). The per-op "a failed op
//! leaves no trace" check is gone: it tested nearcore's receipt atomicity (sim triage, Oct 20).
//!
//! A clean search means no check failed in the cases tried. It is never a proof.

use base64::Engine;
use near_contract_sim::explore::{Audit, Fail, Faults, Gap, Harness, Op, OpDone, Plan, Plans, Skip, plans};
use near_contract_sim::nearcore::near_primitives::hash::hash;
use near_contract_sim::{Balance, Failure, Fixture, ReceiptOutcome, Sim};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const C: &str = "store.near";
const ALICE: &str = "alice.near";
const ROOT: u8 = 0xF0;
/// Every contract method an op reaches; coverage asserts each.
const METHODS: &[&str] = &[
    "bump", "acct_try_insert", "acct_touch", "acct_set_nonce", "acct_remove", "acct_discard",
    "acct_remove_step", "acct_remove_chain", "acct_bulk", "bal_set", "bal_remove", "log_push",
    "log_pop", "blob_set", "blob_remove", "look_insert", "look_try_insert", "look_remove",
    "mem_insert", "mem_remove", "counter", "accounts_len", "acct_contains", "acct_get",
    "look_get", "mem_contains", "accounts_page", "members_page", "balances_page",
];

// =====================================================================================
// World and fixture
// =====================================================================================

/// The sim plus the contract's storage usage with no contract state (code and account).
#[derive(Clone)]
struct World {
    sim: Sim,
    base: u64,
}
impl AsRef<Sim> for World {
    fn as_ref(&self) -> &Sim {
        &self.sim
    }
}
impl AsMut<Sim> for World {
    fn as_mut(&mut self) -> &mut Sim {
        &mut self.sim
    }
}

fn wasm() -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../res/store.wasm");
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e} (run proto-sim-search/build.sh)", p.display()))
}

fn world() -> World {
    let sim = Fixture::new().account(ALICE, Balance::from_near(10_000)).contract(C, wasm()).build();
    let st = state(&sim);
    let base = usage(&sim) - bytes_of(&st);
    World { sim, base }
}

fn state(sim: &Sim) -> BTreeMap<Vec<u8>, Vec<u8>> {
    sim.view_state(C, &[]).into_iter().collect()
}
fn usage(sim: &Sim) -> u64 {
    sim.account(C).expect("contract account").storage_usage
}
fn bytes_of(st: &BTreeMap<Vec<u8>, Vec<u8>>) -> u64 {
    st.iter().map(|(k, v)| (k.len() + v.len() + 40) as u64).sum()
}

// =====================================================================================
// Ops
// =====================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum User {
    A,
    B,
    C,
    D,
}
const USERS: [User; 4] = [User::A, User::B, User::C, User::D];
impl User {
    fn id(self) -> &'static str {
        match self {
            User::A => "alice.near",
            User::B => "bob.near",
            User::C => "carol.near",
            User::D => "dave.near",
        }
    }
}

/// A nested map key: `t{i}` (the names `acct_bulk` writes too), or `x` repeated (up to one past
/// `String`'s 516-byte `MAX_LEN` of `borsh(k)`, so the key-length panic is reached).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Tok {
    T(u8),
    Long(u16),
}
impl Tok {
    fn s(self) -> String {
        match self {
            Tok::T(i) => format!("t{i}"),
            Tok::Long(n) => "x".repeat(n as usize),
        }
    }
}

/// How a mutating method ends (the contract's `mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Mode {
    Plain,
    /// Panic before the wrapper's flush.
    Panic,
    /// `state_write()` by hand, then panic: the runtime must revert the flushed writes.
    FlushPanic,
    /// `state_write()` by hand, then a root change, then the wrapper's flush.
    MidFlush,
}
impl Mode {
    fn code(self) -> u8 {
        match self {
            Mode::Plain => 0,
            Mode::Panic => 1,
            Mode::FlushPanic => 2,
            Mode::MidFlush => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum ReadKind {
    Get,
    Contains,
    Look,
    Member,
    Counter,
    Len,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum StoreOp {
    Bump { by: u64, mode: Mode },
    TryInsert { who: User, nonce: u64, mode: Mode },
    Touch { who: User, mode: Mode },
    SetNonce { who: User, nonce: u64, mode: Mode },
    Remove { who: User, mode: Mode },
    Discard { who: User, mode: Mode },
    RemoveStep { who: User, n: u32, mode: Mode },
    RemoveChain { who: User, n: u32 },
    Bulk { who: User, n: u32, blob: u32, mode: Mode },
    BalSet { who: User, tok: Tok, amount: u64, mode: Mode },
    BalRemove { who: User, tok: Tok, mode: Mode },
    LogPush { who: User, v: u64, mode: Mode },
    LogPop { who: User, mode: Mode },
    BlobSet { who: User, size: u32, mode: Mode },
    BlobRemove { who: User, mode: Mode },
    LookInsert { who: User, v: u64, mode: Mode },
    LookTryInsert { who: User, v: u64, mode: Mode },
    LookRemove { who: User, mode: Mode },
    MemInsert { who: User, mode: Mode },
    MemRemove { who: User, mode: Mode },
    Read { who: User, what: ReadKind },
    /// A page of `accounts`: from the start, from the last page's `next`, or after a key the
    /// plan names (present or not: an absent one must be `CursorGone`).
    AccountsPage { from: From<User>, limit: u32 },
    MembersPage { from: From<User>, limit: u32 },
    BalancesPage { who: User, from: From<Tok>, limit: u32 },
    /// Harness-only, never drawn: writes 8 bytes at `who`'s log slot `slot` straight into the
    /// trie (`Sim::set_state`), submitting nothing. The positive control's planted orphan, as a
    /// step of the plan, so it is a finding after an op on main and on fix/open-gaps alike
    /// (there a check failing on the untouched fixture is a harness error, 32f33955).
    PlantOrphan { who: User, slot: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum From<K> {
    Start,
    Last,
    After(K),
}

impl StoreOp {
    fn call(&self, sim: &Sim) -> (&'static str, Value) {
        use StoreOp::*;
        let m = |mode: &Mode| mode.code();
        match self {
            Bump { by, mode } => ("bump", json!({ "by": by, "mode": m(mode) })),
            TryInsert { who, nonce, mode } => {
                ("acct_try_insert", json!({ "who": who.id(), "nonce": nonce, "mode": m(mode) }))
            }
            Touch { who, mode } => ("acct_touch", json!({ "who": who.id(), "mode": m(mode) })),
            SetNonce { who, nonce, mode } => {
                ("acct_set_nonce", json!({ "who": who.id(), "nonce": nonce, "mode": m(mode) }))
            }
            Remove { who, mode } => ("acct_remove", json!({ "who": who.id(), "mode": m(mode) })),
            Discard { who, mode } => ("acct_discard", json!({ "who": who.id(), "mode": m(mode) })),
            RemoveStep { who, n, mode } => {
                ("acct_remove_step", json!({ "who": who.id(), "n": n, "mode": m(mode) }))
            }
            RemoveChain { who, n } => ("acct_remove_chain", json!({ "who": who.id(), "n": n })),
            Bulk { who, n, blob, mode } => {
                ("acct_bulk", json!({ "who": who.id(), "n": n, "blob": blob, "mode": m(mode) }))
            }
            BalSet { who, tok, amount, mode } => (
                "bal_set",
                json!({ "who": who.id(), "token": tok.s(), "amount": amount, "mode": m(mode) }),
            ),
            BalRemove { who, tok, mode } => {
                ("bal_remove", json!({ "who": who.id(), "token": tok.s(), "mode": m(mode) }))
            }
            LogPush { who, v, mode } => ("log_push", json!({ "who": who.id(), "v": v, "mode": m(mode) })),
            LogPop { who, mode } => ("log_pop", json!({ "who": who.id(), "mode": m(mode) })),
            BlobSet { who, size, mode } => {
                ("blob_set", json!({ "who": who.id(), "size": size, "mode": m(mode) }))
            }
            BlobRemove { who, mode } => ("blob_remove", json!({ "who": who.id(), "mode": m(mode) })),
            LookInsert { who, v, mode } => {
                ("look_insert", json!({ "who": who.id(), "v": v, "mode": m(mode) }))
            }
            LookTryInsert { who, v, mode } => {
                ("look_try_insert", json!({ "who": who.id(), "v": v, "mode": m(mode) }))
            }
            LookRemove { who, mode } => ("look_remove", json!({ "who": who.id(), "mode": m(mode) })),
            MemInsert { who, mode } => ("mem_insert", json!({ "who": who.id(), "mode": m(mode) })),
            MemRemove { who, mode } => ("mem_remove", json!({ "who": who.id(), "mode": m(mode) })),
            Read { who, what } => match what {
                ReadKind::Get => ("acct_get", json!({ "who": who.id() })),
                ReadKind::Contains => ("acct_contains", json!({ "who": who.id() })),
                ReadKind::Look => ("look_get", json!({ "who": who.id() })),
                ReadKind::Member => ("mem_contains", json!({ "who": who.id() })),
                ReadKind::Counter => ("counter", json!({})),
                ReadKind::Len => ("accounts_len", json!({})),
            },
            PlantOrphan { .. } => unreachable!("PlantOrphan submits nothing"),
            AccountsPage { from, limit } => {
                let after = match from {
                    From::Start => None,
                    From::Last => last_cursor(sim, "accounts_page", None),
                    From::After(u) => Some(u.id().to_owned()),
                };
                ("accounts_page", json!({ "after": after, "limit": limit }))
            }
            MembersPage { from, limit } => {
                let after = match from {
                    From::Start => None,
                    From::Last => last_cursor(sim, "members_page", None),
                    From::After(u) => Some(u.id().to_owned()),
                };
                ("members_page", json!({ "after": after, "limit": limit }))
            }
            BalancesPage { who, from, limit } => {
                let after = match from {
                    From::Start => None,
                    From::Last => last_cursor(sim, "balances_page", Some(who.id())),
                    From::After(t) => Some(t.s()),
                };
                ("balances_page", json!({ "who": who.id(), "after": after, "limit": limit }))
            }
        }
    }
}

/// The `next` of the last successful page of `method` (for `who`), read from the chain's own
/// history, so a plan stays meaningful whatever ran before. `None` after a gone cursor or the
/// last page.
fn last_cursor(sim: &Sim, method: &str, who: Option<&str>) -> Option<String> {
    for h in sim.txs().iter().rev() {
        let o = sim.outcome(h);
        for r in o.receipts.iter().rev() {
            if r.executor_id.as_str() != C || r.method_names.first().map(String::as_str) != Some(method) {
                continue;
            }
            let args: Value = serde_json::from_slice(&r.args[0]).unwrap_or(Value::Null);
            if who.is_some_and(|w| args["who"].as_str() != Some(w)) {
                continue;
            }
            if !r.is_success() {
                continue;
            }
            let v: Value = serde_json::from_slice(r.value().unwrap_or(b"null")).unwrap_or(Value::Null);
            return v["next"].as_str().map(str::to_owned);
        }
    }
    None
}

impl Op<World> for StoreOp {
    fn submit(&self, w: &mut World) -> Result<(), Skip> {
        if let StoreOp::PlantOrphan { who, slot } = *self {
            w.sim.set_state(C, &key::vslot(&key::log(who.id()), slot), Some(&[1u8; 8]));
            return Ok(());
        }
        let (method, args) = self.call(&w.sim);
        w.sim.call(ALICE, C, method).args_json(args).submit();
        Ok(())
    }
}

fn mode() -> impl Strategy<Value = Mode> {
    prop_oneof![
        12 => Just(Mode::Plain),
        1 => Just(Mode::Panic),
        1 => Just(Mode::FlushPanic),
        1 => Just(Mode::MidFlush),
    ]
}
fn user() -> impl Strategy<Value = User> {
    // Skewed, so ops meet the same record often enough to reach its later states.
    prop_oneof![6 => Just(User::A), 3 => Just(User::B), 1 => Just(User::C), 1 => Just(User::D)]
}
fn tok() -> impl Strategy<Value = Tok> {
    prop_oneof![
        12 => (0u8..6).prop_map(Tok::T),
        1 => prop_oneof![Just(512u16), Just(513u16), 13u16..64].prop_map(Tok::Long),
    ]
}
fn small() -> impl Strategy<Value = u64> {
    prop_oneof![Just(0u64), 1u64..1000, Just(u64::MAX)]
}
fn step_n() -> impl Strategy<Value = u32> {
    prop_oneof![4 => Just(3u32), 1 => 0u32..3, 4 => 4u32..9, 1 => 9u32..32, 1 => Just(64u32)]
}
fn blob_size() -> impl Strategy<Value = u32> {
    prop_oneof![Just(0u32), Just(1u32), 2u32..300, Just(16_384u32), Just(65_536u32)]
}
fn limit() -> impl Strategy<Value = u32> {
    prop_oneof![1 => Just(0u32), 6 => 1u32..3, 1 => Just(50u32)]
}

fn from<K: Clone + std::fmt::Debug + 'static>(k: impl Strategy<Value = K> + 'static) -> impl Strategy<Value = From<K>> {
    prop_oneof![1 => Just(From::Start), 2 => Just(From::Last), 2 => k.prop_map(From::After)]
}

fn ops() -> impl Strategy<Value = StoreOp> {
    use StoreOp::*;
    let records = prop_oneof![
        3 => (user(), small(), mode()).prop_map(|(who, nonce, mode)| TryInsert { who, nonce, mode }),
        2 => (user(), mode()).prop_map(|(who, mode)| Touch { who, mode }),
        2 => (user(), small(), mode()).prop_map(|(who, nonce, mode)| SetNonce { who, nonce, mode }),
        1 => (user(), mode()).prop_map(|(who, mode)| Remove { who, mode }),
        1 => (user(), mode()).prop_map(|(who, mode)| Discard { who, mode }),
        6 => (user(), step_n(), mode()).prop_map(|(who, n, mode)| RemoveStep { who, n, mode }),
        2 => (user(), step_n()).prop_map(|(who, n)| RemoveChain { who, n }),
        4 => (user(), 0u32..10, blob_size(), mode())
            .prop_map(|(who, n, blob, mode)| Bulk { who, n, blob, mode }),
    ];
    let below = prop_oneof![
        3 => (user(), tok(), small(), mode())
            .prop_map(|(who, tok, amount, mode)| BalSet { who, tok, amount, mode }),
        2 => (user(), tok(), mode()).prop_map(|(who, tok, mode)| BalRemove { who, tok, mode }),
        2 => (user(), small(), mode()).prop_map(|(who, v, mode)| LogPush { who, v, mode }),
        1 => (user(), mode()).prop_map(|(who, mode)| LogPop { who, mode }),
        1 => (user(), blob_size(), mode()).prop_map(|(who, size, mode)| BlobSet { who, size, mode }),
        1 => (user(), mode()).prop_map(|(who, mode)| BlobRemove { who, mode }),
    ];
    let flat = prop_oneof![
        1 => (small(), mode()).prop_map(|(by, mode)| Bump { by, mode }),
        2 => (user(), small(), mode()).prop_map(|(who, v, mode)| LookInsert { who, v, mode }),
        1 => (user(), small(), mode()).prop_map(|(who, v, mode)| LookTryInsert { who, v, mode }),
        1 => (user(), mode()).prop_map(|(who, mode)| LookRemove { who, mode }),
        2 => (user(), mode()).prop_map(|(who, mode)| MemInsert { who, mode }),
        1 => (user(), mode()).prop_map(|(who, mode)| MemRemove { who, mode }),
    ];
    let reads = prop_oneof![
        3 => (user(), prop_oneof![
            3 => Just(ReadKind::Get), 1 => Just(ReadKind::Contains), 1 => Just(ReadKind::Look),
            1 => Just(ReadKind::Member), 1 => Just(ReadKind::Counter), 1 => Just(ReadKind::Len)
        ]).prop_map(|(who, what)| Read { who, what }),
        3 => (from(user()), limit()).prop_map(|(from, limit)| AccountsPage { from, limit }),
        1 => (from(user()), limit()).prop_map(|(from, limit)| MembersPage { from, limit }),
        2 => (user(), from((0u8..6).prop_map(Tok::T)), limit())
            .prop_map(|(who, from, limit)| BalancesPage { who, from, limit }),
    ];
    prop_oneof![5 => records, 4 => below, 2 => flat, 3 => reads]
}

// =====================================================================================
// Keys: the spec's grammar (docs/storage-v2-spec.md, "Keys are paths from the root"), as
// near-sdk/src/state/model.rs `spec_key` writes it. Tags here are all < 128 (one LEB128 byte).
// =====================================================================================

mod key {
    use super::*;
    pub const ENTRIES: u8 = 0x00;
    pub const INDEX: u8 = 0x01;
    pub const SLOTS: u8 = 0x02;
    pub const HEADER: u8 = 0x03;
    pub const FLAG: u8 = 0x00;
    pub const T_COUNTER: u8 = 1;
    pub const T_ACCOUNTS: u8 = 2;
    pub const T_LOOKUP: u8 = 3;
    pub const T_MEMBERS: u8 = 4;
    pub const A_NONCE: u8 = 1;
    pub const A_BAL: u8 = 2;
    pub const A_LOG: u8 = 3;
    pub const A_BLOB: u8 = 4;

    /// `borsh(String)` / `borsh(AccountId)`: u32 LE length, then the bytes.
    pub fn enc_str(s: &str) -> Vec<u8> {
        let mut v = (s.len() as u32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v
    }
    fn h16(enc: &[u8]) -> Vec<u8> {
        hash(enc).0[..16].to_vec()
    }
    fn cat(a: &[u8], b: &[u8]) -> Vec<u8> {
        [a, b].concat()
    }
    /// A map (or set) rooted at `base`.
    pub fn entry(base: &[u8], enc: &[u8]) -> Vec<u8> {
        cat(base, &[&[ENTRIES][..], &h16(enc), enc].concat())
    }
    pub fn index(base: &[u8], enc: &[u8]) -> Vec<u8> {
        cat(base, &[&[INDEX][..], &h16(enc), enc].concat())
    }
    pub fn slot(base: &[u8], i: u32) -> Vec<u8> {
        cat(base, &[&[SLOTS][..], &i.to_le_bytes()].concat())
    }
    pub fn header(base: &[u8]) -> Vec<u8> {
        cat(base, &[HEADER])
    }
    /// `Vector` slots live under the entries kind.
    pub fn vslot(base: &[u8], i: u32) -> Vec<u8> {
        cat(base, &[&[ENTRIES][..], &i.to_le_bytes()].concat())
    }
    pub fn root() -> Vec<u8> {
        vec![ROOT]
    }
    pub fn accounts() -> Vec<u8> {
        vec![ROOT, T_ACCOUNTS]
    }
    pub fn lookup() -> Vec<u8> {
        vec![ROOT, T_LOOKUP]
    }
    pub fn members() -> Vec<u8> {
        vec![ROOT, T_MEMBERS]
    }
    pub fn record(who: &str) -> Vec<u8> {
        entry(&accounts(), &enc_str(who))
    }
    pub fn flag(who: &str) -> Vec<u8> {
        cat(&record(who), &[FLAG])
    }
    pub fn bal(who: &str) -> Vec<u8> {
        cat(&record(who), &[A_BAL])
    }
    pub fn log(who: &str) -> Vec<u8> {
        cat(&record(who), &[A_LOG])
    }
    pub fn blob(who: &str) -> Vec<u8> {
        cat(&record(who), &[A_BLOB])
    }

    fn leb(v: u64, out: &mut Vec<u8>) {
        let mut v = v;
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                out.push(b);
                return;
            }
            out.push(b | 0x80);
        }
    }
    /// Spec, Records: `varint(tag << 2 | wire) varint(len) bytes` in ascending tag order.
    pub fn record_bytes(fields: &[(u8, u8, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (tag, wire, b) in fields {
            leb(((*tag as u64) << 2) | *wire as u64, &mut out);
            leb(b.len() as u64, &mut out);
            out.extend_from_slice(b);
        }
        out
    }
    pub const KIND_MAP: u8 = 0x81;
    pub const KIND_VECTOR: u8 = 0x85;
    pub const KIND_LAZY: u8 = 0x86;
    /// An `Account` record: the nonce, then one kind marker per collection field.
    pub fn account_bytes(nonce: u64) -> Vec<u8> {
        record_bytes(&[
            (A_NONCE, 0, nonce.to_le_bytes().to_vec()),
            (A_BAL, 1, vec![KIND_MAP]),
            (A_LOG, 1, vec![KIND_VECTOR]),
            (A_BLOB, 1, vec![KIND_LAZY]),
        ])
    }
    /// The root carries no markers (a root is never removed).
    pub fn root_bytes(counter: u64) -> Vec<u8> {
        record_bytes(&[(T_COUNTER, 0, counter.to_le_bytes().to_vec())])
    }
    /// A `Map` header: bare `u32` length while the revision is 0, then `len ++ rev`; stored
    /// iff `len != 0 || rev != 0`.
    pub fn map_header(len: u32, rev: u64) -> Option<Vec<u8>> {
        (len != 0 || rev != 0).then(|| {
            let mut v = len.to_le_bytes().to_vec();
            if rev != 0 {
                v.extend_from_slice(&rev.to_le_bytes());
            }
            v
        })
    }
    pub fn blob_of(size: u32) -> Vec<u8> {
        vec![(size % 251) as u8; size as usize]
    }
}

// =====================================================================================
// The model: the logical state, the key-vector orders and revisions, and its concretization.
// =====================================================================================

#[derive(Clone, Debug, Default, PartialEq)]
struct Acct {
    nonce: u64,
    bal: Vec<(String, u64)>,
    bal_rev: u64,
    log: Vec<u64>,
    blob: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
struct Rec {
    a: Acct,
    /// Mid `remove_step`: the record is frozen (writes refused) at what it held when clearing
    /// began; which of its keys are gone yet is the implementation's business (C5 checks it).
    clearing: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Model {
    root_written: bool,
    counter: u64,
    accts: Vec<(String, Rec)>,
    accts_rev: u64,
    look: BTreeMap<String, u64>,
    mems: Vec<String>,
    mems_rev: u64,
}

fn pos<V>(v: &[(String, V)], k: &str) -> Option<usize> {
    v.iter().position(|(x, _)| x == k)
}

impl Acct {
    /// Every key this record owns (its own key excluded), with its bytes.
    fn subtree(&self, who: &str) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut out = BTreeMap::new();
        let b = key::bal(who);
        for (i, (t, v)) in self.bal.iter().enumerate() {
            let e = key::enc_str(t);
            out.insert(key::entry(&b, &e), v.to_le_bytes().to_vec());
            out.insert(key::index(&b, &e), (i as u32).to_le_bytes().to_vec());
            out.insert(key::slot(&b, i as u32), e);
        }
        if let Some(h) = key::map_header(self.bal.len() as u32, self.bal_rev) {
            out.insert(key::header(&b), h);
        }
        let l = key::log(who);
        for (i, v) in self.log.iter().enumerate() {
            out.insert(key::vslot(&l, i as u32), v.to_le_bytes().to_vec());
        }
        if !self.log.is_empty() {
            out.insert(key::header(&l), (self.log.len() as u32).to_le_bytes().to_vec());
        }
        if let Some(blob) = &self.blob {
            out.insert(key::blob(who), [&(blob.len() as u32).to_le_bytes()[..], blob].concat());
        }
        out
    }
    /// Host removes a whole clear of this record takes (spec, Bounded clearing): 3 per map
    /// entry, 1 per `Vector` slot and `Lazy`, 1 per stored header, 3 to finish in a `Map`.
    fn clear_cost(&self) -> u32 {
        let headers = key::map_header(self.bal.len() as u32, self.bal_rev).is_some() as u32
            + !self.log.is_empty() as u32;
        3 * self.bal.len() as u32 + self.log.len() as u32 + self.blob.is_some() as u32 + headers + 3
    }
}

impl Model {
    fn concretize(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut out = BTreeMap::new();
        if self.root_written {
            out.insert(key::root(), key::root_bytes(self.counter));
        }
        let a = key::accounts();
        for (i, (who, rec)) in self.accts.iter().enumerate() {
            let e = key::enc_str(who);
            out.insert(key::index(&a, &e), (i as u32).to_le_bytes().to_vec());
            out.insert(key::slot(&a, i as u32), e);
            if !rec.clearing {
                out.insert(key::record(who), key::account_bytes(rec.a.nonce));
                out.extend(rec.a.subtree(who));
            }
        }
        if let Some(h) = key::map_header(self.accts.len() as u32, self.accts_rev) {
            out.insert(key::header(&a), h);
        }
        for (who, v) in &self.look {
            out.insert(key::entry(&key::lookup(), &key::enc_str(who)), v.to_le_bytes().to_vec());
        }
        let m = key::members();
        for (i, who) in self.mems.iter().enumerate() {
            let e = key::enc_str(who);
            out.insert(key::entry(&m, &e), Vec::new());
            out.insert(key::index(&m, &e), (i as u32).to_le_bytes().to_vec());
            out.insert(key::slot(&m, i as u32), e);
        }
        if let Some(h) = key::map_header(self.mems.len() as u32, self.mems_rev) {
            out.insert(key::header(&m), h);
        }
        out
    }

    fn rec(&self, who: &str) -> Option<&Rec> {
        pos(&self.accts, who).map(|i| &self.accts[i].1)
    }
    fn acct_mut(&mut self, who: &str) -> &mut Acct {
        let i = pos(&self.accts, who).expect("present");
        &mut self.accts[i].1.a
    }
    fn remove_acct(&mut self, who: &str) {
        let i = pos(&self.accts, who).expect("present");
        self.accts.swap_remove(i);
        self.accts_rev += 1;
    }
}

/// What the model expects of one receipt.
enum Exp {
    /// Succeeds, returns this, leaves that state.
    Ret(Value, Model),
    /// Panics with a message containing this.
    Panic(&'static str),
    /// `remove_step`: `Done` or `Pending`, whichever the implementation's budget gives (the
    /// mock's C5 harness pins the exact counts; here `Done` on the first call is predicted from
    /// the record's clear cost, and every call's removes are checked against `n`).
    Step { done: Model, pending: Model, first_call_done: Option<bool>, chain: bool },
}

const CLEARING: &str = "being cleared by `remove_step`";
const KEY_TOO_LONG: &str = "over its declared MAX_LEN";
const PLANNED: &str = "planned panic";
/// `vec![b; size]` past wasm32's `isize::MAX` bytes: Rust's alloc panics before allocating.
const CAPACITY: &str = "capacity overflow";
/// The largest blob the contract can build on wasm32 (`isize::MAX` there).
const WASM_MAX_ALLOC: u64 = i32::MAX as u64;

fn page_slice<K: Clone + PartialEq + Into<Value>>(
    keys: &[K],
    vals: impl Fn(usize) -> Value,
    after: Option<K>,
    limit: u32,
    rev: u64,
) -> Value {
    let start = match after {
        None => 0,
        Some(k) => match keys.iter().position(|x| *x == k) {
            Some(i) => i + 1,
            None => return json!({ "items": [], "next": null, "revision": 0, "gone": true }),
        },
    };
    let end = (start + limit as usize).min(keys.len());
    let items: Vec<Value> = (start..end).map(&vals).collect();
    let next = if limit == 0 || end >= keys.len() { Value::Null } else { keys[end - 1].clone().into() };
    json!({ "items": items, "next": next, "revision": rev, "gone": false })
}

impl Model {
    /// The model's expectation for `method(args)` on this state, the method's ending `mode`
    /// included.
    fn exec(&self, method: &str, a: &Value) -> Result<Exp, String> {
        let exp = self.exec_body(method, a)?;
        let mode = a["mode"].as_u64().unwrap_or(0);
        Ok(match (exp, mode) {
            (Exp::Panic(p), _) => Exp::Panic(p),
            (_, 1 | 2) => Exp::Panic(PLANNED),
            (Exp::Ret(v, mut m), 3) => {
                m.counter = m.counter.wrapping_add(1);
                Exp::Ret(v, m)
            }
            (Exp::Step { mut done, mut pending, first_call_done, chain }, 3) => {
                done.counter = done.counter.wrapping_add(1);
                pending.counter = pending.counter.wrapping_add(1);
                Exp::Step { done, pending, first_call_done, chain }
            }
            (e, _) => e,
        })
    }

    /// What the method body does (before its ending mode).
    fn exec_body(&self, method: &str, a: &Value) -> Result<Exp, String> {
        let who = a["who"].as_str().map(str::to_owned).unwrap_or_default();
        let u = |f: &str| a[f].as_u64().ok_or_else(|| format!("{method}: no u64 arg {f}"));
        let rec = self.rec(&who);
        let clearing = rec.is_some_and(|r| r.clearing);
        let mut m = self.clone();
        let mutating = !matches!(
            method,
            "counter" | "accounts_len" | "acct_contains" | "acct_get" | "look_get" | "mem_contains"
                | "accounts_page" | "members_page" | "balances_page"
        );
        if mutating {
            m.root_written = true;
        }
        let ret = |v: Value, m: Model| Ok(Exp::Ret(v, m));
        let exp = match method {
            "bump" => {
                m.counter = m.counter.wrapping_add(u("by")?);
                let c = m.counter;
                ret(json!(c), m)
            }
            "acct_try_insert" => match rec {
                Some(_) => ret(json!(false), m),
                None => {
                    m.accts.push((who, Rec { a: Acct { nonce: u("nonce")?, ..Default::default() }, clearing: false }));
                    ret(json!(true), m)
                }
            },
            "acct_touch" => match rec {
                Some(r) if r.clearing => Ok(Exp::Panic(CLEARING)),
                Some(r) => ret(json!(r.a.nonce), m),
                None => {
                    m.accts.push((who, Rec { a: Acct::default(), clearing: false }));
                    ret(json!(0), m)
                }
            },
            "acct_set_nonce" => match rec {
                Some(r) if r.clearing => Ok(Exp::Panic(CLEARING)),
                Some(_) => {
                    m.acct_mut(&who).nonce = u("nonce")?;
                    ret(json!(true), m)
                }
                None => ret(json!(false), m),
            },
            "acct_remove" => match rec {
                Some(r) => {
                    let n = r.a.nonce;
                    m.remove_acct(&who);
                    ret(json!(n), m)
                }
                None => ret(Value::Null, m),
            },
            "acct_discard" => match rec {
                Some(_) => {
                    m.remove_acct(&who);
                    ret(json!(true), m)
                }
                None => ret(json!(false), m),
            },
            "acct_remove_step" | "acct_remove_chain" => {
                let n = u("n")?;
                let chain = method == "acct_remove_chain";
                if n == 0 {
                    return Ok(Exp::Panic("n must be > 0"));
                }
                if n < 3 {
                    return Ok(Exp::Panic("must be at least 3"));
                }
                match rec {
                    None => ret(json!(true), m),
                    Some(r) => {
                        let first_call_done = (!r.clearing).then(|| r.a.clear_cost() <= n as u32);
                        let mut done = m.clone();
                        done.remove_acct(&who);
                        let mut pending = m;
                        let i = pos(&pending.accts, &who).unwrap();
                        pending.accts[i].1.clearing = true;
                        Ok(Exp::Step { done, pending, first_call_done, chain })
                    }
                }
            }
            "acct_bulk" => {
                if clearing {
                    return Ok(Exp::Panic(CLEARING));
                }
                if rec.is_none() {
                    m.accts.push((who.clone(), Rec { a: Acct::default(), clearing: false }));
                }
                if u("blob")? > WASM_MAX_ALLOC {
                    return Ok(Exp::Panic(CAPACITY));
                }
                let acct = m.acct_mut(&who);
                for i in 0..u("n")? {
                    let t = format!("t{i}");
                    match acct.bal.iter_mut().find(|(k, _)| *k == t) {
                        Some(e) => e.1 = i + 1,
                        None => acct.bal.push((t, i + 1)),
                    }
                    acct.log.push(i + 100);
                }
                acct.blob = Some(key::blob_of(u("blob")? as u32));
                let n = acct.nonce;
                ret(json!(n), m)
            }
            "bal_set" | "bal_remove" | "log_push" | "log_pop" | "blob_set" | "blob_remove" => {
                let Some(r) = rec else {
                    let absent = match method {
                        "bal_set" | "bal_remove" | "log_pop" => Value::Null,
                        _ => json!(false),
                    };
                    return ret(absent, m);
                };
                if r.clearing {
                    return Ok(Exp::Panic(CLEARING));
                }
                let tok = a["token"].as_str().unwrap_or_default().to_owned();
                if matches!(method, "bal_set" | "bal_remove") && tok.len() > 512 {
                    return Ok(Exp::Panic(KEY_TOO_LONG));
                }
                if method == "blob_set" && u("size")? > WASM_MAX_ALLOC {
                    return Ok(Exp::Panic(CAPACITY));
                }
                let acct = m.acct_mut(&who);
                let v = match method {
                    "bal_set" => {
                        let amount = u("amount")?;
                        match acct.bal.iter_mut().find(|(k, _)| *k == tok) {
                            Some(e) => {
                                e.1 = amount;
                                json!(false)
                            }
                            None => {
                                acct.bal.push((tok, amount));
                                json!(true)
                            }
                        }
                    }
                    "bal_remove" => match pos(&acct.bal, &tok) {
                        Some(i) => {
                            let (_, v) = acct.bal.swap_remove(i);
                            acct.bal_rev += 1;
                            json!(v)
                        }
                        None => Value::Null,
                    },
                    "log_push" => {
                        acct.log.push(u("v")?);
                        json!(true)
                    }
                    "log_pop" => acct.log.pop().map_or(Value::Null, |v| json!(v)),
                    "blob_set" => {
                        acct.blob = Some(key::blob_of(u("size")? as u32));
                        json!(true)
                    }
                    "blob_remove" => {
                        acct.blob = None;
                        json!(true)
                    }
                    _ => unreachable!(),
                };
                ret(v, m)
            }
            "look_insert" => {
                m.look.insert(who, u("v")?);
                ret(Value::Null, m)
            }
            "look_try_insert" => {
                if m.look.contains_key(&who) {
                    ret(json!(false), m)
                } else {
                    m.look.insert(who, u("v")?);
                    ret(json!(true), m)
                }
            }
            "look_remove" => {
                let v = m.look.remove(&who);
                ret(v.map_or(Value::Null, |v| json!(v)), m)
            }
            "mem_insert" => {
                if m.mems.contains(&who) {
                    ret(json!(false), m)
                } else {
                    m.mems.push(who);
                    ret(json!(true), m)
                }
            }
            "mem_remove" => match m.mems.iter().position(|x| *x == who) {
                Some(i) => {
                    m.mems.swap_remove(i);
                    m.mems_rev += 1;
                    ret(json!(true), m)
                }
                None => ret(json!(false), m),
            },
            "counter" => ret(json!(self.counter), m),
            "accounts_len" => ret(json!(self.accts.len()), m),
            "acct_contains" => ret(json!(rec.is_some()), m),
            "look_get" => ret(self.look.get(&who).map_or(Value::Null, |v| json!(v)), m),
            "mem_contains" => ret(json!(self.mems.contains(&who)), m),
            "acct_get" => ret(
                rec.map_or(Value::Null, |r| {
                    json!({
                        "nonce": r.a.nonce,
                        "balances_len": r.a.bal.len(),
                        "balances": r.a.bal,
                        "log_len": r.a.log.len(),
                        "log": r.a.log,
                        "blob_len": r.a.blob.as_ref().map(|b| b.len()),
                    })
                }),
                m,
            ),
            "accounts_page" => {
                let keys: Vec<String> = self.accts.iter().map(|(k, _)| k.clone()).collect();
                let v = page_slice(
                    &keys,
                    |i| json!([self.accts[i].0, self.accts[i].1.a.nonce]),
                    a["after"].as_str().map(str::to_owned),
                    u("limit")? as u32,
                    self.accts_rev,
                );
                ret(v, m)
            }
            "members_page" => {
                let v = page_slice(
                    &self.mems,
                    |i| json!(self.mems[i]),
                    a["after"].as_str().map(str::to_owned),
                    u("limit")? as u32,
                    self.mems_rev,
                );
                ret(v, m)
            }
            "balances_page" => {
                let empty = Acct::default();
                let acct = rec.map_or(&empty, |r| &r.a);
                let keys: Vec<String> = acct.bal.iter().map(|(k, _)| k.clone()).collect();
                let v = page_slice(
                    &keys,
                    |i| json!([acct.bal[i].0, acct.bal[i].1]),
                    a["after"].as_str().map(str::to_owned),
                    u("limit")? as u32,
                    acct.bal_rev,
                );
                ret(v, m)
            }
            other => Err(format!("model: unknown method {other}")),
        }?;
        Ok(exp)
    }
}

// =====================================================================================
// Replay: every contract receipt of the case, in execution order, through the model.
// =====================================================================================

#[derive(Debug)]
enum Actual {
    Ok(Value),
    Promise,
    /// Gas ran out (natural or forced), or a forced fault: legitimate anywhere; must leave
    /// no trace.
    Gas(String),
    Panic(String),
    Other(String),
}

fn actual(r: &ReceiptOutcome) -> Actual {
    if r.is_success() {
        return match r.value() {
            Some([]) => Actual::Ok(Value::Null),
            Some(b) => Actual::Ok(serde_json::from_slice(b).unwrap_or_else(|_| json!({ "raw": format!("{b:?}") }))),
            None => Actual::Promise,
        };
    }
    if let Some(f) = &r.fault {
        return Actual::Gas(format!("forced {f:?}"));
    }
    match r.failure() {
        Some(Failure::OutOfGas) => Actual::Gas("out of gas".into()),
        Some(Failure::GasLimitExceeded) => Actual::Gas("gas limit exceeded".into()),
        Some(Failure::Panic(m)) => Actual::Panic(m),
        f => Actual::Other(format!("{f:?}")),
    }
}

/// One receipt's account in the replay: what ran, what the model said, what happened.
fn step(m: &Model, r: &ReceiptOutcome, at: &str) -> Result<Model, String> {
    let method = r.method_names[0].clone();
    let args: Value = serde_json::from_slice(&r.args[0]).map_err(|e| format!("{at}: args: {e}"))?;
    // The outcome first: a receipt that ran out of gas left no trace, and the model is never
    // asked what it expected (a scan's type-max edge, `Bulk { n: u32::MAX }`, would make the
    // model loop 4 billion times for a receipt that died at the gas ceiling).
    let act = actual(r);
    let call = format!("{at}: {method}({args})");
    match &act {
        Actual::Gas(_) => return Ok(m.clone()),
        Actual::Other(f) => return Err(format!("{call}: failed outside gas or a panic: {f}")),
        _ => {}
    }
    let exp = m.exec(&method, &args)?;
    match (exp, act) {
        (_, Actual::Gas(_)) => Ok(m.clone()),
        (_, Actual::Other(f)) => Err(format!("{call}: failed outside gas or a panic: {f}")),
        (Exp::Panic(p), Actual::Panic(msg)) => {
            if msg.contains(p) {
                Ok(m.clone())
            } else {
                Err(format!("{call}: panicked with {msg:?}, the model expected a panic containing {p:?}"))
            }
        }
        (Exp::Panic(p), a) => Err(format!("{call}: the model expected a panic ({p:?}), got {a:?}")),
        (_, Actual::Panic(msg)) => Err(format!("{call}: unexpected panic: {msg}")),
        (Exp::Ret(v, next), Actual::Ok(got)) => {
            if method == "acct_get" && v != got && is_clearing(m, &args) {
                check_clearing_view(m, &args, &got).map_err(|e| format!("{call}: {e}"))?;
                return Ok(next);
            }
            if method == "balances_page" && is_clearing(m, &args) {
                return Ok(next); // a clearing record's nested map is mid-clear; not modelled
            }
            if v != got {
                return Err(format!("{call}: returned {got}, the model expected {v}"));
            }
            Ok(next)
        }
        (Exp::Ret(v, _), Actual::Promise) => Err(format!("{call}: returned a promise, the model expected {v}")),
        (Exp::Step { done, pending, first_call_done, chain }, got) => {
            let is_done = match (&got, chain) {
                (Actual::Ok(Value::Bool(b)), _) => *b,
                (Actual::Promise, true) => false,
                (g, _) => return Err(format!("{call}: remove_step returned {g:?}")),
            };
            if let Some(fits) = first_call_done {
                if fits != is_done {
                    return Err(format!(
                        "{call}: first remove_step call returned done={is_done}, the record's clear cost \
                         {} vs n says done={fits}",
                        m.rec(args["who"].as_str().unwrap()).unwrap().a.clear_cost()
                    ));
                }
            }
            let n = args["n"].as_u64().unwrap();
            let removes = r.storage_ops().removes;
            if removes > n {
                return Err(format!("{call}: issued {removes} host removes, over the budget n = {n}"));
            }
            // Liveness: on a present record every call with n >= 3 removes something (the first
            // Pending call at least moves the record to its flag key).
            if removes == 0 {
                return Err(format!("{call}: returned done={is_done} on a present record with 0 host removes"));
            }
            Ok(if is_done { done } else { pending })
        }
        
    }
}

fn is_clearing(m: &Model, args: &Value) -> bool {
    args["who"].as_str().and_then(|w| m.rec(w)).is_some_and(|r| r.clearing)
}

/// `acct_get` of a record mid-clear: the nonce is exact (its bytes sit verbatim at the flag
/// key); the nested map holds a subset of what it held, the log a prefix, the blob all or none.
fn check_clearing_view(m: &Model, args: &Value, got: &Value) -> Result<(), String> {
    let a = &m.rec(args["who"].as_str().unwrap()).unwrap().a;
    if got["nonce"] != json!(a.nonce) {
        return Err(format!("clearing record read nonce {}, expected {}", got["nonce"], a.nonce));
    }
    let bal: Vec<(String, u64)> = serde_json::from_value(got["balances"].clone()).map_err(|e| e.to_string())?;
    if bal.iter().any(|e| !a.bal.contains(e)) || got["balances_len"] != json!(bal.len()) {
        return Err(format!("clearing record's balances {bal:?} not a subset of {:?}", a.bal));
    }
    let log: Vec<u64> = serde_json::from_value(got["log"].clone()).map_err(|e| e.to_string())?;
    if !a.log.starts_with(&log) || got["log_len"] != json!(log.len()) {
        return Err(format!("clearing record's log {log:?} not a prefix of {:?}", a.log));
    }
    let blob = &got["blob_len"];
    if !blob.is_null() && *blob != json!(a.blob.as_ref().map(|b| b.len())) {
        return Err(format!("clearing record's blob_len {blob}, had {:?}", a.blob.as_ref().map(|b| b.len())));
    }
    Ok(())
}

/// The contract's receipts over the whole chain, in execution order (refunds left out). The
/// search draws isolated steps (`Gap::Settle`, `.repeats(false)`), but a scan's or a curve's
/// variants and a shrunk plan need not be, and a chained `remove_step` spans blocks, so
/// execution order, not plan order, is the truth.
fn executed(sim: &Sim) -> Vec<ReceiptOutcome> {
    sim.receipts_in(..)
        .into_iter()
        .filter(|r| r.executor_id.as_str() == C && !r.is_refund() && !r.method_names.is_empty())
        .collect()
}

fn replay(sim: &Sim) -> Result<Model, String> {
    let mut m = Model::default();
    for (i, r) in executed(sim).iter().enumerate() {
        m = step(&m, r, &format!("receipt {i} at #{}", r.block_height))?;
    }
    Ok(m)
}

// =====================================================================================
// State checks
// =====================================================================================

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Names a key for a failure message.
fn describe(k: &[u8]) -> String {
    for u in USERS {
        let w = u.id();
        let rk = key::record(w);
        if k == rk {
            return format!("accounts[{w}] record");
        }
        if k == key::flag(w) {
            return format!("accounts[{w}] flag key");
        }
        if k.starts_with(&key::bal(w)) {
            return format!("accounts[{w}].balances/{}", hex(&k[key::bal(w).len()..]));
        }
        if k.starts_with(&key::log(w)) {
            return format!("accounts[{w}].log/{}", hex(&k[key::log(w).len()..]));
        }
        if k == key::blob(w) {
            return format!("accounts[{w}].blob");
        }
        if k.starts_with(&rk) {
            return format!("under accounts[{w}]: {}", hex(&k[rk.len()..]));
        }
    }
    hex(k)
}

fn diff(expected: &BTreeMap<Vec<u8>, Vec<u8>>, actual: &BTreeMap<Vec<u8>, Vec<u8>>) -> Vec<String> {
    let keys: BTreeSet<&Vec<u8>> = expected.keys().chain(actual.keys()).collect();
    let mut out = Vec::new();
    for k in keys {
        match (expected.get(k), actual.get(k)) {
            (Some(e), Some(a)) if e != a => {
                out.push(format!("{}: stored {} expected {}", describe(k), hex(a), hex(e)))
            }
            (Some(e), None) => out.push(format!("{}: MISSING (expected {})", describe(k), hex(e))),
            (None, Some(a)) => out.push(format!("{}: EXTRA {} ({} bytes)", describe(k), hex(&a[..a.len().min(24)]), a.len())),
            _ => {}
        }
    }
    out
}

/// The trie equals the model's concretization; records mid-clear are checked by C5's rules.
fn check_state(m: &Model, actual: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<(), String> {
    let expected = m.concretize();
    let clearing: Vec<(&str, Vec<u8>, &Acct)> = m
        .accts
        .iter()
        .filter(|(_, r)| r.clearing)
        .map(|(w, r)| (w.as_str(), key::record(w), &r.a))
        .collect();
    let mut rest = BTreeMap::new();
    let mut subs: BTreeMap<&str, BTreeMap<Vec<u8>, Vec<u8>>> = BTreeMap::new();
    for (k, v) in actual {
        match clearing.iter().find(|(_, rk, _)| k.starts_with(rk)) {
            Some((w, _, _)) => {
                subs.entry(w).or_default().insert(k.clone(), v.clone());
            }
            None => {
                rest.insert(k.clone(), v.clone());
            }
        }
    }
    let d = diff(&expected, &rest);
    if !d.is_empty() {
        return Err(format!("trie != model ({} keys differ):\n  {}", d.len(), d.join("\n  ")));
    }
    for (w, rk, a) in &clearing {
        let sub = subs.remove(w).unwrap_or_default();
        if sub.contains_key(rk) {
            return Err(format!("accounts[{w}] is clearing but its record key is present"));
        }
        match sub.get(&key::flag(w)) {
            Some(b) if *b == key::account_bytes(a.nonce) => {}
            Some(b) => return Err(format!("accounts[{w}] flag key holds {}, expected the record's bytes", hex(b))),
            None => return Err(format!("accounts[{w}] is clearing but has no flag key")),
        }
        let snap = a.subtree(w);
        let headers = [key::header(&key::bal(w)), key::header(&key::log(w))];
        for (k, v) in &sub {
            if *k == key::flag(w) {
                continue;
            }
            match snap.get(k) {
                Some(s) if s == v || headers.contains(k) => {}
                Some(s) => return Err(format!("clearing {}: {} changed from {}", describe(k), hex(v), hex(s))),
                None => return Err(format!("clearing accounts[{w}]: key {} it did not hold when clearing began", describe(k))),
            }
        }
    }
    Ok(())
}

/// C4: the runtime's storage usage is the empty contract's plus key + value + 40 per key.
fn check_accounting(w: &World, actual: &BTreeMap<Vec<u8>, Vec<u8>>) -> Result<(), String> {
    let (u, want) = (usage(&w.sim), w.base + bytes_of(actual));
    if u != want {
        return Err(format!("storage_usage {u} != base {} + trie bytes {} = {want}", w.base, bytes_of(actual)));
    }
    Ok(())
}

/// G2/C2 through the host, bypassing the v2 cache: nothing under an absent record's path.
fn check_probes(w: &World, m: &Model) -> Result<(), String> {
    let mut keys = Vec::new();
    for u in USERS {
        let who = u.id();
        if m.rec(who).is_none() {
            keys.extend([
                key::record(who),
                key::flag(who),
                key::header(&key::bal(who)),
                key::header(&key::log(who)),
                key::vslot(&key::log(who), 0),
                key::blob(who),
            ]);
        }
    }
    if keys.is_empty() {
        return Ok(());
    }
    let b64: Vec<String> = keys.iter().map(|k| base64::engine::general_purpose::STANDARD.encode(k)).collect();
    let has: Vec<bool> = w.sim.view(C, "probe_has", json!({ "keys": b64 })).json();
    match keys.iter().zip(&has).find(|(_, h)| **h) {
        Some((k, _)) => Err(format!("storage_has_key true under an absent record: {}", describe(k))),
        None => Ok(()),
    }
}

/// C6 walk property, on top of the exact per-page check: chained pages of one revision return
/// no key twice.
fn check_walks(sim: &Sim) -> Result<(), String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut last: Option<(Option<String>, u64)> = None; // (next, revision)
    {
        for r in &executed(sim) {
            if r.method_names[0] != "accounts_page" || !r.is_success() {
                continue;
            }
            let args: Value = serde_json::from_slice(&r.args[0]).unwrap_or_default();
            let p: Value = serde_json::from_slice(r.value().unwrap_or(b"null")).unwrap_or_default();
            if p["gone"] == json!(true) {
                last = None;
                continue;
            }
            let after = args["after"].as_str().map(str::to_owned);
            let rev = p["revision"].as_u64().unwrap_or(0);
            let chained = after.is_some() && last.as_ref().is_some_and(|(n, r)| *n == after && *r == rev);
            if !chained {
                seen.clear();
            }
            for it in p["items"].as_array().into_iter().flatten() {
                let k = it[0].as_str().unwrap_or_default().to_owned();
                if !seen.insert(k.clone()) {
                    return Err(format!("page walk at revision {rev} returned {k} twice"));
                }
            }
            last = Some((p["next"].as_str().map(str::to_owned), rev));
        }
    }
    Ok(())
}

fn at_rest(w: &World) -> Result<(), String> {
    let m = replay(&w.sim)?;
    let actual = state(&w.sim);
    check_state(&m, &actual)?;
    check_probes(w, &m)?;
    check_walks(&w.sim)
}

/// Runtime sanity (nearcore's storage accounting), kept apart from the SDK's properties.
fn runtime_accounting(w: &World) -> Result<(), String> {
    check_accounting(w, &state(&w.sim))
}


fn is_remove_step(op: &StoreOp) -> bool {
    matches!(op, StoreOp::RemoveStep { .. } | StoreOp::RemoveChain { .. })
}

/// C5 per op: a `remove_step` op only ever removes keys under its record (plus the flag-key
/// move), never adds one under it other than the flag key. It compares the op's before and
/// after worlds, so it needs the op alone: overlapped, it is not judged (harness.md, Checks).
fn remove_step_shrinks(d: &OpDone<'_, StoreOp, World>) -> Result<(), Fail> {
    let (StoreOp::RemoveStep { who, .. } | StoreOp::RemoveChain { who, .. }) = d.op() else {
        unreachable!("per_op_for(is_remove_step)")
    };
    if !d.isolated() {
        return Err(Fail::not_judged("overlapped: before/after of a shared world"));
    }
    let rk = key::record(who.id());
    let under = |sim: &Sim| -> BTreeSet<Vec<u8>> { state(sim).into_keys().filter(|k| k.starts_with(&rk)).collect() };
    let (b, a) = (under(&d.before().sim), under(&d.world().sim));
    let added: Vec<String> = a.difference(&b).filter(|k| **k != key::flag(who.id())).map(|k| describe(k)).collect();
    if !added.is_empty() {
        return Err(Fail::new(format!("remove_step added keys under the record: {added:?}")));
    }
    Ok(())
}

/// Behaviour classes the case's contract receipts reached. NOT a property (it never fails) and
/// NOT counted once per case: a check runs on every execution, so these counts include shrink
/// candidates, a faulted case's targeting baseline and case 0's determinism rerun (sim triage
/// §4). Distinct-case coverage is `Audit::reached` per method site, printed beside it.
mod tally {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    static T: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());
    pub fn add(k: impl Into<String>) {
        *T.lock().unwrap().entry(k.into()).or_default() += 1;
    }
    pub fn take() -> BTreeMap<String, u64> {
        std::mem::take(&mut *T.lock().unwrap())
    }
    pub fn print(t: &BTreeMap<String, u64>) {
        println!("behaviour classes, in receipts over every execution (cases + shrink candidates + fault baselines + case 0's rerun; an upper bound, not distinct cases):");
        for (k, v) in t {
            println!("  {k}: {v}");
        }
    }
}

const PROBE: &str = "probe: behaviour classes (never fails; counts include reruns)";

fn classify(w: &World) -> Result<(), String> {
    let mut m = Model::default();
    for (i, r) in executed(&w.sim).iter().enumerate() {
        let meth = r.method_names[0].as_str();
        let args: Value = serde_json::from_slice(&r.args[0]).unwrap_or_default();
        let mode = args["mode"].as_u64().unwrap_or(0);
        match actual(r) {
            Actual::Gas(why) if why.starts_with("forced") => tally::add(format!("forced fault at {meth}")),
            Actual::Gas(_) => tally::add(format!("natural out of gas: {meth}")),
            Actual::Panic(msg) => {
                let what = if msg.contains(CLEARING) {
                    "write below a clearing record refused"
                } else if msg.contains("planned panic after flush") {
                    let o = r.storage_ops();
                    if o.writes + o.removes > 0 {
                        tally::add("  of which the hand flush reached the host before the panic");
                    }
                    "panic after a hand flush (reverted)"
                } else if msg.contains(PLANNED) {
                    "panic before the flush"
                } else if msg.contains(KEY_TOO_LONG) {
                    "nested key over MAX_LEN refused"
                } else if msg.contains("at least 3") || msg.contains("n must be") {
                    "remove_step n < 3 refused"
                } else {
                    "other panic"
                };
                tally::add(what);
            }
            Actual::Ok(v) => {
                if meth.ends_with("_page") {
                    if v["gone"] == json!(true) {
                        tally::add(format!("{meth}: CursorGone"));
                    } else if args["after"].is_string() {
                        tally::add(format!("{meth}: continued from a cursor"));
                    }
                }
                if is_remove_step_method(meth) {
                    if r.storage_ops().removes > 0 {
                        tally::add("remove_step on a present record (budget and liveness judged)");
                    }
                    tally::add(format!("{meth}: {}", if v == json!(true) { "Done" } else { "Pending" }));
                }
                if meth == "acct_get" && !v.is_null() && is_clearing(&m, &args) {
                    tally::add("acct_get of a clearing record");
                }
                if mode == 3 {
                    tally::add("hand flush mid-call, then root change");
                }
            }
            Actual::Promise => tally::add(format!("{meth}: Pending (chained)")),
            Actual::Other(_) => tally::add("other failure"),
        }
        // The model before the next receipt; a divergence is the property check's to report.
        match step(&m, r, &format!("receipt {i}")) {
            Ok(next) => m = next,
            Err(_) => break,
        }
    }
    Ok(())
}

fn is_remove_step_method(m: &str) -> bool {
    m == "acct_remove_step" || m == "acct_remove_chain"
}

fn harness() -> Harness<StoreOp, World> {
    Harness::new(world())
        .audits([C])
        .at_rest("model and trie agree", |w: &World| at_rest(w))
        .per_op_for(is_remove_step, "remove_step only shrinks the record", |d: &OpDone<StoreOp, World>| remove_step_shrinks(d))
        .at_rest("runtime: storage_usage is base + key + value + 40 per key", |w: &World| runtime_accounting(w))
        .at_end(PROBE, |w: &World| classify(w))
}

/// Isolated steps: `Gap::Settle` after each, and no repeat bursts (a repeat puts its copies in
/// one block whatever the gaps: plan.rs:667, D23), so the per-op check can judge every op and the
/// reducer never meets a `Blocks(0)` to leave behind (sim triage §3).
fn search_plans() -> Plans<StoreOp> {
    let p = plans(ops()).len(1..=12).gaps(Just(Gap::Settle)).repeats(false);
    // `SEARCH_GAS=wide`: log-uniform gas on every step, so far more steps run out mid-method.
    match std::env::var("SEARCH_GAS").as_deref() {
        Ok("wide") => p.gas(near_contract_sim::explore::gas::wide()),
        _ => p,
    }
}

// =====================================================================================
// Campaign accounting the 8e8642db summary doesn't print
// =====================================================================================

mod campaign {
    use near_contract_sim::explore::{Audit, Harness, Op, Plan, World};
    use proptest::strategy::{NewTree, Strategy, ValueTree};
    use proptest::test_runner::TestRunner;
    use serde::Serialize;
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::fmt::Debug;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// Labels of checks that are not SDK properties: shown apart from the property table.
    const NOT_PROPERTIES: [&str; 3] = ["runtime:", "harness:", "probe:"];

    /// Prints each check's judgements as `Audit::add` sums them: once per counted case (shrink
    /// candidates, a faulted case's targeting baseline and case 0's rerun are not counted).
    /// `Audit::judged` is crate-private at 8e8642db and the summary prints no per-check line
    /// (fix/open-gaps 37f7f588 adds a not-judged line); `Report`'s JSON carries the map.
    pub fn print_checks<O: Clone + Debug>(audit: &Audit<O>) {
        let v: Value = serde_json::from_str(&audit.report().json()).expect("report JSON");
        let n = |j: &Value, f: &str| j[f].as_u64().unwrap_or(0);
        let rows: Vec<(String, &Value)> =
            v["judged"].as_object().map(|m| m.iter().map(|(k, j)| (k.clone(), j)).collect()).unwrap_or_default();
        for (title, props) in [("SDK properties", true), ("not SDK properties (runtime sanity, harness, probes)", false)] {
            println!("checks, {title}, over the {} counted case(s):", audit.cases());
            for (k, j) in rows.iter().filter(|(k, _)| props != NOT_PROPERTIES.iter().any(|p| k.starts_with(p))) {
                let why = j["why"].as_str().map(|w| format!(" (first: {w})")).unwrap_or_default();
                println!(
                    "  {k}: {} ok, {} failed, {} not judged, {} idle{why}",
                    n(j, "ok"), n(j, "failed"), n(j, "unjudged"), n(j, "idle")
                );
            }
        }
        println!("incomplete cases: {} of {}", audit.incomplete().len(), audit.cases());
    }

    /// What the plan strategy drew: each distinct plan, and when the first and the last new
    /// one were drawn.
    pub struct Seen<O> {
        keys: BTreeSet<String>,
        pub plans: Vec<Plan<O>>,
        first: Option<Instant>,
        last: Option<Instant>,
    }

    impl<O> Seen<O> {
        /// The mean case time with shrinking left out: a search draws case k's plan as case k
        /// starts, and shrinks only after its seed's cases, so the time from the first new plan
        /// to the last over the cases between (case 0 runs twice, its determinism check).
        pub fn per_case(&self) -> String {
            match (self.first, self.last, self.plans.len()) {
                (Some(a), Some(b), n) if n > 1 => {
                    format!("{:.0} ms per case, shrinking excluded (cases 0..{})", (b - a).as_secs_f64() * 1000.0 / (n - 1) as f64, n - 1)
                }
                _ => "per case: too few cases".into(),
            }
        }
    }

    /// The plan strategy, keeping each distinct plan it draws. A search calls `new_tree` once per
    /// case, and once more per finding it shrinks (the case seed regenerates the same plan), so
    /// plans are kept once, by their JSON.
    pub struct Drawn<S, O> {
        pub inner: S,
        pub seen: Arc<Mutex<Seen<O>>>,
    }

    impl<S: Debug, O> Debug for Drawn<S, O> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.inner.fmt(f)
        }
    }

    impl<S: Strategy<Value = Plan<O>>, O: Serialize + Clone + Debug> Strategy for Drawn<S, O> {
        type Tree = S::Tree;
        type Value = Plan<O>;
        fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
            let tree = self.inner.new_tree(runner)?;
            let plan = tree.current();
            let key = serde_json::to_string(&plan).unwrap_or_default();
            let mut g = self.seen.lock().unwrap();
            if g.keys.insert(key) {
                g.plans.push(plan);
                let now = Instant::now();
                g.first.get_or_insert(now);
                g.last = Some(now);
            }
            Ok(tree)
        }
    }

    pub fn drawn<S, O>(inner: S) -> (Drawn<S, O>, Arc<Mutex<Seen<O>>>) {
        let seen = Arc::new(Mutex::new(Seen { keys: BTreeSet::new(), plans: vec![], first: None, last: None }));
        (Drawn { inner, seen: seen.clone() }, seen)
    }

    /// "faults: N drawn, M landed", which the 8e8642db summary doesn't print (sim triage §1: the
    /// old "N injected, M drew a fault nothing fits" line has no counterpart on main). Of the
    /// distinct plans drawn, those carrying a sampled fault, and those whose fault the run
    /// resolves to a receipt (`Run::plan`: "a sampled injection resolved to the receipt it landed
    /// on"; one that fits nothing is dropped). Each faulted plan runs once more, here.
    pub fn faults<O: Op<W>, W: World>(h: &Harness<O, W>, plans: &[Plan<O>]) -> String {
        let faulted: Vec<&Plan<O>> = plans.iter().filter(|p| p.steps.iter().any(|s| !s.inject.is_empty())).collect();
        let landed = faulted.iter().filter(|p| h.run(p).plan().steps.iter().any(|s| !s.inject.is_empty())).count();
        format!(
            "faults: {} drawn, {landed} landed (of {} distinct plans drawn; the rest fit no receipt their case spawned: a transaction's first receipt is its caller's gas, faults.md)",
            faulted.len(),
            plans.len()
        )
    }
}

fn assert_coverage(audit: &Audit<StoreOp>) {
    println!("coverage, distinct cases reaching each method (Audit::reached):");
    for m in METHODS {
        let site = format!("{C}::{m}");
        println!("  {site}: {}", audit.reached(&site));
    }
    for m in METHODS {
        audit.assert_reached(&format!("{C}::{m}"));
    }
}

fn assert_no_findings(audit: &Audit<StoreOp>) {
    assert!(audit.found().is_empty() && audit.errors().is_empty(), "findings or errors:\n{audit:#?}");
}

// =====================================================================================
// Tests
// =====================================================================================

/// A hand-written plan through every op, the failure modes included (so it is `refused`: its
/// `FlushPanic` step fails on purpose, and every failure is still judged by the model); the smoke
/// test of the harness itself.
fn witness() -> Plan<StoreOp> {
    use Mode::Plain;
    use StoreOp::*;
    let (a, b) = (User::A, User::B);
    Plan::new()
        .op(Bump { by: 5, mode: Plain })
        .op(TryInsert { who: a, nonce: 7, mode: Plain })
        .op(Bulk { who: a, n: 4, blob: 300, mode: Plain })
        .op(BalSet { who: a, tok: Tok::T(9), amount: 3, mode: Plain })
        .op(BalRemove { who: a, tok: Tok::T(1), mode: Plain })
        .op(LogPush { who: a, v: 1, mode: Plain })
        .op(LogPop { who: a, mode: Plain })
        .op(Touch { who: b, mode: Plain })
        .op(SetNonce { who: b, nonce: 9, mode: Mode::MidFlush })
        .op(LookInsert { who: a, v: 1, mode: Plain })
        .op(LookTryInsert { who: a, v: 2, mode: Plain })
        .op(MemInsert { who: a, mode: Plain })
        .op(MemInsert { who: b, mode: Plain })
        .op(AccountsPage { from: From::Start, limit: 1 })
        .op(AccountsPage { from: From::Last, limit: 1 })
        .op(RemoveStep { who: a, n: 3, mode: Plain })
        .op(BalSet { who: a, tok: Tok::T(0), amount: 1, mode: Plain })
        .op(Read { who: a, what: ReadKind::Get })
        .op(RemoveStep { who: a, n: 4, mode: Plain })
        .op(RemoveChain { who: a, n: 3 })
        .op(BalancesPage { who: b, from: From::After(Tok::T(0)), limit: 2 })
        .op(Remove { who: b, mode: Mode::FlushPanic })
        .op(Discard { who: b, mode: Plain })
        .op(MemRemove { who: a, mode: Plain })
        .op(MembersPage { from: From::After(User::B), limit: 50 })
        .op(LookRemove { who: a, mode: Plain })
        .op(BlobSet { who: a, size: 1, mode: Plain })
        .op(BlobRemove { who: a, mode: Plain })
        .op(Read { who: a, what: ReadKind::Len })
        .refused()
}

#[test]
fn smoke_witness_is_clean() {
    let t = std::time::Instant::now();
    let run = harness().run(&witness());
    println!("{}", run.trees());
    println!("smoke: {} steps in {:?}", witness().steps.len(), t.elapsed());
    run.assert_clean();
}

/// The checks can fail: after alice's record is removed, a key planted under her log slot 7 (a
/// removed record's path) is found by the model check, as an extra key.
#[test]
fn positive_control_orphan_key_is_found() {
    let plan = Plan::new()
        .op(StoreOp::Bulk { who: User::A, n: 2, blob: 3, mode: Mode::Plain })
        .op(StoreOp::Remove { who: User::A, mode: Mode::Plain });
    harness().run(&plan).assert_clean();
    // A step that submits nothing produces no block, so the model check judges the plant only
    // at the next rest point: a read transaction after it.
    let planted = plan
        .op(StoreOp::PlantOrphan { who: User::A, slot: 7 })
        .op(StoreOp::Read { who: User::A, what: ReadKind::Counter });
    let run = harness().run(&planted);
    let f = run.assert_found("model and trie agree", "world");
    assert!(f.message.contains("accounts[alice.near].log") && f.message.contains("EXTRA"), "{run}");
    println!("positive control: {}", f.message.lines().take(2).collect::<Vec<_>>().join(" / "));
}

#[test]
fn search() {
    let h = harness();
    let (strategy, drawn) = campaign::drawn(search_plans());
    let t = std::time::Instant::now();
    let audit = h.search(strategy).cases(256).run();
    let wall = t.elapsed();
    println!("{audit}");
    let per_case = wall.as_secs_f64() / audit.cases().max(1) as f64;
    println!("wall time: {wall:?} for {} cases, {:.3} s per case (shrinking included)", audit.cases(), per_case);
    campaign::print_checks(&audit);
    println!("{}", drawn.lock().unwrap().per_case());
    let plans = std::mem::take(&mut drawn.lock().unwrap().plans);
    println!("{}", campaign::faults(&h, &plans));
    tally::print(&tally::take());
    assert_coverage(&audit);
    assert_no_findings(&audit);
}

/// Every fault of the default set at every receipt of each witness (the self-calls of
/// `acct_remove_chain` are the only spawned receipts), plus each step at the caller's least
/// gas and the wasm's gas constants, and each signed number's edges.
#[test]
fn scan_witnesses() {
    let chain = Plan::new()
        .op(StoreOp::Bulk { who: User::A, n: 3, blob: 64, mode: Mode::Plain })
        .op(StoreOp::RemoveChain { who: User::A, n: 3 })
        .op(StoreOp::Read { who: User::A, what: ReadKind::Get })
        .op(StoreOp::TryInsert { who: User::A, nonce: 1, mode: Mode::Plain });
    let t = std::time::Instant::now();
    let audit = harness().scan(&[witness(), chain]).faults(Faults::default()).run();
    println!("{audit}");
    println!("wall time: {:?} for {} cases", t.elapsed(), audit.cases());
    println!("cases reaching acct_remove_chain: {}", audit.reached(&format!("{C}::acct_remove_chain")));
    campaign::print_checks(&audit);
    tally::print(&tally::take());
    assert_no_findings(&audit);
}

/// The mutating shapes: one witness whose every step's caller gas is climbed.
fn walk_plan() -> Plan<StoreOp> {
    use Mode::Plain;
    use StoreOp::*;
    let (a, b) = (User::A, User::B);
    Plan::new()
        .op(Bulk { who: a, n: 3, blob: 300, mode: Plain })
        .op(BalSet { who: a, tok: Tok::T(7), amount: 3, mode: Plain })
        .op(BalRemove { who: a, tok: Tok::T(0), mode: Plain })
        .op(LogPop { who: a, mode: Plain })
        .op(TryInsert { who: b, nonce: 1, mode: Plain })
        .op(MemInsert { who: b, mode: Plain })
        .op(LookInsert { who: b, v: 1, mode: Plain })
        .op(SetNonce { who: a, nonce: 2, mode: Mode::MidFlush })
        .op(RemoveStep { who: a, n: 4, mode: Plain })
        .op(RemoveStep { who: a, n: 5, mode: Plain })
        .op(RemoveChain { who: a, n: 3 })
        .op(Remove { who: b, mode: Plain })
        .op(MemRemove { who: b, mode: Plain })
}

/// Per-case time without the search around it: the same N plans (drawn from a fixed seed by the
/// search's own strategy), run one by one. For comparing build profiles under a noisy machine.
#[test]
#[ignore = "timing: TIMING_CASES (default 128) fixed plans, run one by one"]
fn timing_probe() {
    // On a big stack, as a search draws: the plan strategy recurses deeply.
    std::thread::Builder::new().stack_size(512 << 20).spawn(|| {
        use proptest::strategy::ValueTree;
        use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
        let n: usize = std::env::var("TIMING_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(128);
        let mut r = TestRunner::new_with_rng(Config::default(), TestRng::from_seed(RngAlgorithm::ChaCha, &[7u8; 32]));
        let plans: Vec<_> = (0..n).map(|_| search_plans().new_tree(&mut r).unwrap().current()).collect();
        let h = harness();
        let t = std::time::Instant::now();
        let steps: usize = plans.iter().map(|p| { let _ = h.run(p); p.steps.len() }).sum();
        let el = t.elapsed();
        println!("TIMING {n} plans ({steps} steps) in {:.1} s: {:.0} ms per case", el.as_secs_f64(), el.as_secs_f64() * 1000.0 / n as f64);
    }).unwrap().join().unwrap();
}

/// Replaces the hand gas walk: each step's caller gas climbed by curves (explore.md, Curves:
/// 1 Tgas to the ceiling, coarse to fine, halving where outcomes change), the rest of the plan
/// running on whatever each rung left, under every check. A single-receipt op that runs out of
/// gas commits nothing (nearcore's atomicity), so what this judges is where outcomes change:
/// for `RemoveChain`, how many links committed.
#[test]
fn caller_gas_curves() {
    let h = harness();
    let plan = walk_plan();
    h.run(&plan).assert_clean();
    let t = std::time::Instant::now();
    let mut cases = 0;
    for s in 0..plan.steps.len() {
        let audit = h.curve(&plan, s, "caller gas").run();
        cases += audit.cases();
        println!("step {s} ({:?}): {} case(s)", plan.steps[s].act.op().unwrap(), audit.cases());
        for row in audit.curves().iter().filter(|r| r.site.starts_with(C)) {
            println!("  {row}");
        }
        if !audit.found().is_empty() || !audit.errors().is_empty() {
            panic!("step {s}:\n{audit}");
        }
    }
    println!("caller-gas curves: {cases} case(s) in {:?}", t.elapsed());
    tally::print(&tally::take());
}
