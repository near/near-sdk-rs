//! Real-runtime gas for the storage v2 prototype against v1 `store`, on near-contract-sim.
//!
//! One contract source (`../contract`) built three ways: v1 (`store::IterableMap` /
//! `store::LookupMap` under a Borsh `STATE` root), v2 with the 16-byte entry hash, v2 with the
//! 8-byte one. Each scenario gets a fresh `Sim` holding one contract with one collection
//! populated. Populations above a handful of keys are written as state records in exactly the
//! layout the contract writes (checked byte for byte against keys the contract seeded itself,
//! as `tests/trie_gas.rs` does in the simulator), so 100k keys take seconds.
//!
//! `gas_report` (ignored) prints the tables in GAS_MEASUREMENTS.md.

use near_contract_sim::nearcore::near_primitives::hash::hash;
use near_contract_sim::{Balance, Sim, StateRecord, TxOutcome};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const C: &str = "bench.near";
const ALICE: &str = "alice.near";
/// Keys the contract seeds itself before the records flood; their bytes are the reference.
const SEEDED: u32 = 8;
/// Distinct keys measured per operation; the spread among them is trie placement.
const K: u32 = 7;
const V2_ROOT: u8 = 0xF0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Variant {
    V1,
    V2H16,
    V2H8,
    /// v2 h16 with the Oct 18 methods (`ext` feature); used only for the rows added then.
    V2H16X,
    /// v2 h16 with a `Map<AccountId, ()>` (`zst` feature); used only for its remove rows.
    V2H16Z,
}
use Variant::*;

impl Variant {
    fn wasm(self) -> Vec<u8> {
        let name = match self {
            V1 => "v1",
            V2H16 => "v2_h16",
            V2H8 => "v2_h8",
            V2H16X => "v2_h16_ext",
            V2H16Z => "v2_h16_zst",
        };
        // `GAS_RES_DIR` points at another build's wasms (the per-commit bisection in
        // GAS_MEASUREMENTS.md, Oct 18); default `../res`.
        let dir = std::env::var("GAS_RES_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../res"));
        let p = dir.join(format!("{name}.wasm"));
        std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e} (run proto-gas/build.sh)", p.display()))
    }
    fn label(self, which: &str) -> String {
        match (self, which) {
            (V1, "map") => "v1 store::IterableMap".into(),
            (V1, "lookup") => "v1 store::LookupMap".into(),
            (V1, "blobs") => "v1 store::IterableMap".into(),
            (V2H16 | V2H16X, w) => format!("v2 {} (h16)", if w == "lookup" { "LookupMap" } else { "Map" }),
            (V2H8, w) => format!("v2 {} (h8)", if w == "lookup" { "LookupMap" } else { "Map" }),
            _ => unreachable!(),
        }
    }
    fn h(self) -> usize {
        if self == V2H8 { 8 } else { 16 }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The contract's `gen_id`.
fn gen_id(i: u32) -> String {
    format!("{}.near", hex(&hash(format!("u{i}").as_bytes()).0[..8]))
}

fn borsh_id(i: u32) -> Vec<u8> {
    borsh::to_vec(&gen_id(i)).unwrap()
}

fn sha(b: &[u8]) -> [u8; 32] {
    hash(b).0
}

// ------------------------------------------------------------------ cost receipt

#[derive(Default, Clone, Copy, Debug)]
struct Cost {
    total: u64,
    base: u64,
    bytes: u64,
    trie: u64,
    sha: u64,
    wasm: u64,
    load: u64,
    other: u64,
    nodes: u64,
    reads: u64,
    writes: u64,
    removes: u64,
    has_keys: u64,
    sha_base: u64,
}

fn cost(o: &TxOutcome) -> Cost {
    o.assert_success();
    let mut c = Cost::default();
    for r in o.receipts.iter().filter(|r| !r.is_refund() && r.executor_id.as_str() == C) {
        c.total += r.gas_burnt.as_gas();
        for (k, v) in &r.gas_profile {
            let v = *v;
            if k == "TOUCHING_TRIE_NODE" || k == "READ_CACHED_TRIE_NODE" {
                c.trie += v;
            } else if k.starts_with("STORAGE_") && k.ends_with("_BASE") {
                c.base += v;
            } else if k.starts_with("STORAGE_") {
                c.bytes += v;
            } else if k.starts_with("SHA256") {
                c.sha += v;
                if k == "SHA256_BASE" {
                    c.sha_base += v;
                }
            } else if k == "WASM_INSTRUCTION" {
                c.wasm += v;
            } else if k.starts_with("CONTRACT_LOADING") {
                c.load += v;
            }
        }
        let s = r.storage_ops();
        c.nodes += s.trie_nodes + s.cached_nodes;
        c.reads += s.reads;
        c.writes += s.writes;
        c.removes += s.removes;
        c.has_keys += s.has_keys;
    }
    c.other = c.total - (c.base + c.bytes + c.trie + c.sha + c.wasm + c.load);
    c
}

fn t(g: u64) -> String {
    format!("{:.3}", g as f64 / 1e12)
}
fn gg(g: u64) -> String {
    format!("{:.1}", g as f64 / 1e9)
}

const HEADER: &str = "| collection | keys | op | Tgas median (min-max) | storage base | per-byte | trie nodes (n) | sha256 | wasm | load | other | r/w/d/h |\n|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|";

fn row(label: &str, n: u32, op: &str, cs: &[Cost]) -> String {
    let mut v = cs.to_vec();
    v.sort_by_key(|c| c.total);
    let m = v[v.len() / 2];
    let range = if v.len() > 1 {
        format!(" ({}-{})", t(v[0].total), t(v[v.len() - 1].total))
    } else {
        String::new()
    };
    format!(
        "| {label} | {n} | {op} | {}{range} | {} | {} | {} ({}) | {} | {} | {} | {} | {}/{}/{}/{} |",
        t(m.total),
        gg(m.base),
        gg(m.bytes),
        gg(m.trie),
        m.nodes,
        gg(m.sha),
        gg(m.wasm),
        gg(m.load),
        gg(m.other),
        m.reads,
        m.writes,
        m.removes,
        m.has_keys
    )
}

// ------------------------------------------------------------------ world and layout

fn call(sim: &mut Sim, method: &str, args: Value) -> TxOutcome {
    sim.call(ALICE, C, method).args_json(args).transact()
}

/// Other accounts holding contract data (`GAS_OTHERS`, default 0): each gets a `STATE` entry and
/// one map entry, as `contract_state_records` in the simulator's tests does for every tenth of
/// its mainnet-like accounts. All contract data shares one trie column, so these deepen the
/// path to every key of `bench.near`. Only the data is written (no account records): only the
/// contract-data column shapes the path.
fn others() -> u32 {
    std::env::var("GAS_OTHERS").map(|s| s.parse().unwrap()).unwrap_or(0)
}

fn other_data(n: u32) -> Vec<StateRecord> {
    (0..n)
        .flat_map(|i| {
            let r = hash(format!("pop{i}").as_bytes()).0;
            let id = match i % 3 {
                0 => hex(&r),
                1 => format!("user{}.near", u32::from_le_bytes([r[0], r[1], r[2], 0])),
                _ => format!("{}.tg", hex(&r[..4])),
            };
            let account_id: near_contract_sim::AccountId = id.parse().unwrap();
            let mut k = b"a".to_vec();
            k.extend(borsh::to_vec(&id).unwrap());
            [(b"STATE".to_vec(), r.to_vec()), (k, (i as u128).to_le_bytes().to_vec())].map(|(k, v)| {
                StateRecord::Data { account_id: account_id.clone(), data_key: k.into(), value: v.into() }
            })
        })
        .collect()
}

fn world(v: Variant) -> Sim {
    let mut sim = Sim::new();
    if others() > 0 {
        sim.apply_state_records(other_data(others()));
    }
    sim.create_account(ALICE, Balance::from_near(1_000));
    sim.create_account(C, Balance::from_near(100_000));
    sim.deploy(C, v.wasm());
    let o = match v {
        V1 => call(&mut sim, "new", json!({})),
        _ => call(&mut sim, "set_owner", json!({})),
    };
    o.assert_success();
    sim
}

/// The records the contract writes for keys `range` of `which`, in the layout under test.
/// For v1 `map` the length lives in `STATE` and is patched separately.
fn layout(v: Variant, which: &str, range: std::ops::Range<u32>, len: u32) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut out = vec![];
    for i in range {
        let kb = borsh_id(i);
        let val = (i as u128).to_le_bytes().to_vec();
        match (v, which) {
            (V1, "map") => {
                out.push((sha(&[b"mm".as_slice(), &kb].concat()).to_vec(), [val, i.to_le_bytes().to_vec()].concat()));
                out.push(([b"mv".as_slice(), &i.to_le_bytes()].concat(), kb));
            }
            (V1, "lookup") => out.push(([b"l".as_slice(), &kb].concat(), val)),
            (_, "map") => {
                let h = &sha(&kb)[..v.h()];
                let base = [V2_ROOT, 3];
                out.push(([&base[..], &[0], h, &kb].concat(), val));
                out.push(([&base[..], &[1], h, &kb].concat(), i.to_le_bytes().to_vec()));
                out.push(([&base[..], &[2], &i.to_le_bytes()].concat(), kb));
            }
            (_, "lookup") => {
                let h = &sha(&kb)[..v.h()];
                out.push(([&[V2_ROOT, 4, 0][..], h, &kb].concat(), val));
            }
            (V2H16Z, "units") => {
                // A `()` value has no bytes: the entry is written with an empty value.
                let h = &sha(&kb)[..v.h()];
                let base = [V2_ROOT, 9];
                out.push(([&base[..], &[0], h, &kb].concat(), vec![]));
                out.push(([&base[..], &[1], h, &kb].concat(), i.to_le_bytes().to_vec()));
                out.push(([&base[..], &[2], &i.to_le_bytes()].concat(), kb));
            }
            (V2H16X, "accounts") => {
                // `Account { nonce: i, balances }`: field key `04` (tag 1, scalar) len 8 nonce,
                // then `09` (tag 2, marker) len 1 and the `Map` kind 0x81 (API round 5, wire
                // type since Verification round 2); the empty map writes no keys.
                let rec = account_rec(i as u64, true);
                let h = &sha(&kb)[..v.h()];
                let base = [V2_ROOT, 6];
                out.push(([&base[..], &[0], h, &kb].concat(), rec));
                out.push(([&base[..], &[1], h, &kb].concat(), i.to_le_bytes().to_vec()));
                out.push(([&base[..], &[2], &i.to_le_bytes()].concat(), kb));
            }
            _ => unreachable!(),
        }
    }
    if v != V1 && which == "map" {
        out.push((vec![V2_ROOT, 3, 3], len.to_le_bytes().to_vec()));
    }
    if which == "accounts" {
        out.push((vec![V2_ROOT, 6, 3], len.to_le_bytes().to_vec()));
    }
    if which == "units" {
        out.push((vec![V2_ROOT, 9, 3], len.to_le_bytes().to_vec()));
    }
    out
}

/// An `Account` record's bytes; `marker: false` is a record an older schema without
/// `balances` wrote (no `09 01 81`).
fn account_rec(nonce: u64, marker: bool) -> Vec<u8> {
    let mut rec = [&[4u8, 8][..], &nonce.to_le_bytes()].concat();
    if marker {
        rec.extend_from_slice(&[9, 1, 0x81]);
    }
    rec
}

/// `accounts`' record key for `gen_id(i)`.
fn account_key(i: u32) -> Vec<u8> {
    let kb = borsh_id(i);
    [&[V2_ROOT, 6, 0][..], &sha(&kb)[..16], &kb].concat()
}

/// A3 first-write rows (Oct 20), on the `ext` wasm beside `n` accounts. Each first write goes
/// into an empty nested collection below a record that already exists (created in an earlier
/// call), through `at_mut`, then the same handle path writes again:
/// - `balances` of a record created by `try_insert` (its marker is present);
/// - `balances` of a record an older schema wrote (seeded as state, no marker);
/// - `Profile.note`, a `Lazy<u64>`, set twice.
fn a3_ops(n: u32) -> Vec<(String, Vec<Cost>)> {
    let v = V2H16X;
    let mut sim = world(v);
    populate(&mut sim, v, "accounts", n);
    let (mut first, mut second, mut update, mut first_nm, mut second_nm, mut lazy1, mut lazy2) =
        (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
    for j in 0..K {
        let id = gen_id(n + 6000 + j);
        call(&mut sim, "account_try_insert", json!({"id": id, "nonce": 5})).assert_success();
        first.push(cost(&call(&mut sim, "account_balance_set", json!({"id": id, "token": "t0", "v": "1"}))));
        second.push(cost(&call(&mut sim, "account_balance_set", json!({"id": id, "token": "t1", "v": "2"}))));
        update.push(cost(&call(&mut sim, "account_balance_set", json!({"id": id, "token": "t1", "v": "3"}))));
        // An account record without the `balances` marker (written before the field existed).
        let i = n + 7000 + j;
        sim.set_state(C, &account_key(i), Some(&account_rec(5, false)));
        let id = gen_id(i);
        first_nm.push(cost(&call(&mut sim, "account_balance_set", json!({"id": id, "token": "t0", "v": "1"}))));
        second_nm.push(cost(&call(&mut sim, "account_balance_set", json!({"id": id, "token": "t1", "v": "2"}))));
        let id = gen_id(n + 8000 + j);
        call(&mut sim, "profile_try_insert", json!({"id": id})).assert_success();
        lazy1.push(cost(&call(&mut sim, "profile_note_set", json!({"id": id, "v": "1"}))));
        lazy2.push(cost(&call(&mut sim, "profile_note_set", json!({"id": id, "v": "2"}))));
    }
    vec![
        ("at_mut: 1st write into empty nested Map (record has marker)".to_string(), first),
        ("at_mut: 2nd write, new key, same nested Map".to_string(), second),
        ("at_mut: update existing key in nested Map".to_string(), update),
        ("at_mut: 1st write into empty nested Map (record without marker)".to_string(), first_nm),
        ("at_mut: 2nd write into that Map".to_string(), second_nm),
        ("at_mut: Lazy<u64>::set, value absent".to_string(), lazy1),
        ("at_mut: Lazy<u64>::set, value present".to_string(), lazy2),
    ]
}

/// `Map<AccountId, ()>` holding `n` keys (`zst` wasm): `remove` of a present non-last key, of
/// an absent key, and `discard` of a present non-last key (what `Set::remove` pays). K distinct
/// keys each.
fn zst_ops(n: u32) -> Vec<(&'static str, Vec<Cost>)> {
    let v = V2H16Z;
    let mut sim = world(v);
    populate(&mut sim, v, "units", n);
    let (mut rm, mut miss, mut dis) = (vec![], vec![], vec![]);
    for j in 0..K {
        let o = call(&mut sim, "unit_remove", json!({"id": gen_id((j * 7919 + n / 2 + 1) % (n - 1))}));
        assert!(o.json::<bool>(), "unit_remove of a present key");
        rm.push(cost(&o));
        let o = call(&mut sim, "unit_remove", json!({"id": gen_id(n + 1000 + j)}));
        assert!(!o.json::<bool>(), "unit_remove of an absent key");
        miss.push(cost(&o));
        let o = call(&mut sim, "unit_discard", json!({"id": gen_id((j * 7919 + n / 3 + 1) % (n - 1))}));
        assert!(o.json::<bool>(), "unit_discard of a present key");
        dis.push(cost(&o));
    }
    vec![
        ("Map<AccountId, ()>::remove (non-last)", rm),
        ("Map<AccountId, ()>::remove, absent key", miss),
        ("Map<AccountId, ()>::discard (non-last) = Set::remove", dis),
    ]
}

/// Prints the `Map<AccountId, ()>` remove rows, for the slim-down before/after of Oct 22.
/// `cargo +1.95.0 test --test storage_v2_gas zst_rows -- --include-ignored --nocapture`
#[test]
#[ignore = "zero-sized-value remove before/after helper; run with --nocapture"]
fn zst_rows() {
    let n: u32 = std::env::var("GAS_A3_N").map(|s| s.parse().unwrap()).unwrap_or(10_000);
    println!("{HEADER}");
    for (op, cs) in zst_ops(n) {
        println!("{}", row("v2 (h16, zst)", n, op, &cs));
    }
}

/// Prints the `remove_step` rows of both maps, for the slim-down before/after of Oct 22 (round 3).
/// `cargo +1.95.0 test --test storage_v2_gas remove_step_rows -- --include-ignored --nocapture`
#[test]
#[ignore = "remove_step before/after helper; run with --nocapture"]
fn remove_step_rows() {
    let n: u32 = std::env::var("GAS_A3_N").map(|s| s.parse().unwrap()).unwrap_or(10_000);
    println!("{HEADER}");
    for (op, cs) in remove_step_ops(n) {
        if cs.is_empty() {
            println!("| v2 (h16, ext) | {n} | {op} | | | | | | | | | |");
        } else {
            println!("{}", row("v2 (h16, ext)", n, &op, &cs));
        }
    }
}

/// Prints only the A3 rows and the rows the fix could touch (insert/update/remove at 10k on
/// both v2 maps, the ext rows), for the before/after comparison of Oct 20.
/// `cargo +1.95.0 test --test storage_v2_gas a3_rows -- --include-ignored --nocapture`
#[test]
#[ignore = "A3 before/after helper; run with --nocapture"]
fn a3_rows() {
    let n: u32 = std::env::var("GAS_A3_N").map(|s| s.parse().unwrap()).unwrap_or(10_000);
    println!("{HEADER}");
    for which in ["map", "lookup"] {
        for (op, cs) in ops(V2H16, which, n) {
            println!("{}", row(&V2H16.label(which), n, op, &cs));
        }
    }
    for (op, cs) in ext_ops(n) {
        if cs.is_empty() {
            println!("| v2 (h16, ext) | {n} | {op} | | | | | | | | | |");
        } else {
            println!("{}", row("v2 (h16, ext)", n, &op, &cs));
        }
    }
    for (op, cs) in a3_ops(n) {
        println!("{}", row("v2 (h16, ext)", n, &op, &cs));
    }
    let mut sim = world(V2H16);
    println!("{}", row("v2 root (h16)", 0, "deposit (new account via entry().or_default(), 1st token)", &[cost(&call(&mut sim, "deposit", json!({"token": "usdt", "amount": "5"})))]));
    println!("{}", row("v2 root (h16)", 1, "deposit (existing account, new token)", &[cost(&call(&mut sim, "deposit", json!({"token": "usdc", "amount": "5"})))]));
    println!("{}", row("v2 root (h16)", 1, "balance_of (2 hops)", &[cost(&call(&mut sim, "balance_of", json!({"id": ALICE, "token": "usdt"})))]));
}

fn data(kv: Vec<(Vec<u8>, Vec<u8>)>) -> impl Iterator<Item = StateRecord> {
    kv.into_iter().map(|(k, v)| StateRecord::Data {
        account_id: C.parse().unwrap(),
        data_key: k.into(),
        value: v.into(),
    })
}

/// Populates `which` with `n` keys: `SEEDED` through the contract, checked byte for byte
/// against `layout`, then the rest as state records.
fn populate(sim: &mut Sim, v: Variant, which: &str, n: u32) {
    call(sim, "seed", json!({"start": 0, "n": SEEDED, "which": which})).assert_success();
    let truth: BTreeMap<Vec<u8>, Vec<u8>> = sim.view_state(C, b"").into_iter().collect();
    let mine: BTreeMap<_, _> = layout(v, which, 0..SEEDED, SEEDED).into_iter().collect();
    for (k, val) in &truth {
        if k.as_slice() == b"STATE" || k.as_slice() == [V2_ROOT] {
            continue;
        }
        assert_eq!(mine.get(k), Some(val), "{v:?} {which}: contract wrote {} = {}", hex(k), hex(val));
    }
    assert_eq!(truth.len(), mine.len() + 1, "{v:?} {which}: key count");
    if n > SEEDED {
        sim.apply_state_records(data(layout(v, which, SEEDED..n, n)));
        if v == V1 && which == "map" {
            let mut st = sim.state(C, b"STATE").unwrap();
            let pat = [2u8, 0, 0, 0, b'm', b'v'];
            let at = st.windows(6).position(|w| w == pat).expect("map prefix in STATE") - 4;
            assert_eq!(&st[at..at + 4], &SEEDED.to_le_bytes());
            st[at..at + 4].copy_from_slice(&n.to_le_bytes());
            sim.set_state(C, b"STATE", Some(&st));
        }
    }
    if which == "map" {
        assert_eq!(sim.view(C, "map_len", json!({})).json::<u32>(), n);
    }
    if which == "units" {
        // No getter: the remove rows check each key they remove was present.
        assert_eq!(sim.view(C, "units_len", json!({})).json::<u32>(), n);
        return;
    }
    if which == "accounts" {
        assert_eq!(sim.view(C, "accounts_len", json!({})).json::<u32>(), n);
    }
    let getter = match which {
        "map" => "map_get",
        "accounts" => "account_nonce",
        _ => "lookup_get",
    };
    let got: Option<String> = sim.view(C, getter, json!({"id": gen_id(n - 1)})).json();
    assert_eq!(got, Some((n - 1).to_string()), "{v:?} {which}: last flooded key");
}

// ------------------------------------------------------------------ scenarios

/// Insert-new, update and remove on `which` holding `n` keys: K distinct keys each.
fn ops(v: Variant, which: &str, n: u32) -> Vec<(&'static str, Vec<Cost>)> {
    let mut sim = world(v);
    populate(&mut sim, v, which, n);
    let (ins, rm) = if which == "map" { ("map_insert", "map_remove") } else { ("lookup_insert", "lookup_remove") };
    let mut new = vec![];
    for j in 0..K {
        new.push(cost(&call(&mut sim, ins, json!({"id": gen_id(n + 1000 + j), "v": "5"}))));
    }
    let mut upd = vec![];
    for j in 0..K {
        let i = (j * 7919 + 3) % (n - 1);
        upd.push(cost(&call(&mut sim, ins, json!({"id": gen_id(i), "v": "999"}))));
    }
    let mut del = vec![];
    for j in 0..K {
        let i = (j * 7919 + n / 2 + 1) % (n - 1);
        del.push(cost(&call(&mut sim, rm, json!({"id": gen_id(i)}))));
    }
    vec![("insert new", new), ("update existing", upd), ("remove (non-last)", del)]
}

/// Rows added on Oct 18, on the `ext` wasm (h16): `page_with(after, 50)` of `map` holding `n`
/// keys; `Map::try_insert` of a collection-owning record into `accounts` holding `n` records;
/// `LookupMap::try_insert` of the same record (beside those `n` records); `remove_step` of a
/// record owning a 10-entry map, at the minimum budget (first and finishing call) and in one
/// call. K distinct keys each.
fn ext_ops(n: u32) -> Vec<(String, Vec<Cost>)> {
    let v = V2H16X;
    let mut out = vec![];
    {
        let mut sim = world(v);
        populate(&mut sim, v, "map", n);
        let mut pg = vec![];
        for j in 0..K {
            let i = (j * 7919 + 3) % (n - 60);
            let o = call(&mut sim, "map_page", json!({"after": gen_id(i), "limit": 50}));
            let page: Value = o.json();
            assert_eq!(page["items"].as_array().unwrap().len(), 50);
            assert_eq!(page["items"][0][0].as_str().unwrap(), gen_id(i + 1));
            assert_eq!(page["next"].as_str().unwrap(), gen_id(i + 50));
            pg.push(cost(&o));
        }
        out.push(("page_with(after, 50) of Map<AccountId, u128>".to_string(), pg));
        out.push(("ping (same wasm)".to_string(), vec![cost(&call(&mut sim, "ping", json!({})))]));
    }
    let mut sim = world(v);
    populate(&mut sim, v, "accounts", n);
    let mut ti = vec![];
    for j in 0..K {
        ti.push(cost(&call(&mut sim, "account_try_insert", json!({"id": gen_id(n + 1000 + j), "nonce": 5}))));
    }
    out.push(("Map::try_insert record owning a Map (new key)".to_string(), ti));
    let mut lti = vec![];
    for j in 0..K {
        lti.push(cost(&call(&mut sim, "laccount_try_insert", json!({"id": gen_id(n + 4000 + j), "nonce": 5}))));
    }
    out.push(("LookupMap::try_insert record owning a Map (new key)".to_string(), lti));
    const BAL: u32 = 10;
    let (mut first, mut last, mut whole, mut calls, mut totals) = (vec![], vec![], vec![], vec![], vec![]);
    for j in 0..K {
        let id = gen_id(n + 2000 + j);
        call(&mut sim, "account_fill", json!({"id": id, "n": BAL})).assert_success();
        let mut cs = vec![];
        loop {
            let o = call(&mut sim, "account_remove_step", json!({"id": id, "n": 3}));
            let done: bool = o.json();
            let c = cost(&o);
            assert!(c.removes <= 3, "remove_step(3) made {} removes", c.removes);
            cs.push(c);
            if done {
                break;
            }
            assert!(cs.len() < 100);
        }
        assert!(sim.view(C, "account_nonce", json!({"id": id})).json::<Option<String>>().is_none());
        calls.push(cs.len());
        totals.push(Cost { total: cs.iter().map(|c| c.total).sum(), removes: cs.iter().map(|c| c.removes).sum(), ..Default::default() });
        first.push(cs[0]);
        last.push(*cs.last().unwrap());
        let id = gen_id(n + 3000 + j);
        call(&mut sim, "account_fill", json!({"id": id, "n": BAL})).assert_success();
        let o = call(&mut sim, "account_remove_step", json!({"id": id, "n": 64}));
        assert!(o.json::<bool>(), "n = 64 finishes a 10-entry record in one call");
        whole.push(cost(&o));
    }
    calls.sort();
    out.push((format!("remove_step(n=3), first call (Pending), record + {BAL}-entry Map"), first));
    out.push((format!("remove_step(n=3), finishing call (Done); calls to finish {}-{}", calls[0], calls[calls.len() - 1]), last));
    totals.sort_by_key(|c| c.total);
    let m = totals[totals.len() / 2];
    out.push((format!("remove_step(n=3), whole sequence: total {} Tgas, {} removes", t(m.total), m.removes), vec![]));
    out.push((format!("remove_step(n=64), one call (Done), record + {BAL}-entry Map"), whole));
    out
}

/// `remove_step` rows for both maps (slim-down pass A, round 3, Oct 22), on the `ext` wasm with
/// `accounts` holding `n` records: as in `ext_ops`, a record owning a 10-entry map removed at the
/// minimum budget (first call, finishing call, whole sequence) and in one call (n=64), through
/// `accounts` (`Map`) and `laccounts` (`LookupMap`). K distinct keys each.
fn remove_step_ops(n: u32) -> Vec<(String, Vec<Cost>)> {
    const BAL: u32 = 10;
    let mut sim = world(V2H16X);
    populate(&mut sim, V2H16X, "accounts", n);
    let mut out = vec![];
    for (m, label) in [("account", "Map"), ("laccount", "LookupMap")] {
        let (mut first, mut last, mut whole, mut calls, mut totals) = (vec![], vec![], vec![], vec![], vec![]);
        for j in 0..K {
            let id = gen_id(n + 5000 + j);
            call(&mut sim, &format!("{m}_fill"), json!({"id": id, "n": BAL})).assert_success();
            let mut cs = vec![];
            loop {
                let o = call(&mut sim, &format!("{m}_remove_step"), json!({"id": id, "n": 3}));
                let done: bool = o.json();
                let c = cost(&o);
                assert!(c.removes <= 3, "remove_step(3) made {} removes", c.removes);
                cs.push(c);
                if done {
                    break;
                }
                assert!(cs.len() < 100);
            }
            assert!(sim.view(C, &format!("{m}_nonce"), json!({"id": id})).json::<Option<String>>().is_none());
            calls.push(cs.len());
            totals.push(Cost { total: cs.iter().map(|c| c.total).sum(), removes: cs.iter().map(|c| c.removes).sum(), ..Default::default() });
            first.push(cs[0]);
            last.push(*cs.last().unwrap());
            let id = gen_id(n + 6000 + j);
            call(&mut sim, &format!("{m}_fill"), json!({"id": id, "n": BAL})).assert_success();
            let o = call(&mut sim, &format!("{m}_remove_step"), json!({"id": id, "n": 64}));
            assert!(o.json::<bool>(), "n = 64 finishes a 10-entry record in one call");
            whole.push(cost(&o));
        }
        calls.sort();
        out.push((format!("{label}::remove_step(n=3), first call (Pending), record + {BAL}-entry Map"), first));
        out.push((format!("{label}::remove_step(n=3), finishing call (Done); calls to finish {}-{}", calls[0], calls[calls.len() - 1]), last));
        totals.sort_by_key(|c| c.total);
        let m = totals[totals.len() / 2];
        out.push((format!("{label}::remove_step(n=3), whole sequence: total {} Tgas, {} removes", t(m.total), m.removes), vec![]));
        out.push((format!("{label}::remove_step(n=64), one call (Done), record + {BAL}-entry Map"), whole));
    }
    out
}

/// Remove a small non-last key whose last neighbor holds `neighbor` bytes.
fn remove_beside(v: Variant, neighbor: u32) -> Cost {
    let mut sim = world(v);
    call(&mut sim, "blob_insert", json!({"id": gen_id(1), "size": 16})).assert_success();
    call(&mut sim, "blob_insert", json!({"id": gen_id(2), "size": neighbor})).assert_success();
    cost(&call(&mut sim, "blob_remove", json!({"id": gen_id(1)})))
}

fn sizes() -> Vec<u32> {
    std::env::var("GAS_SIZES")
        .unwrap_or_else(|_| "1000,10000,100000".into())
        .split(',')
        .map(|s| s.trim().parse().unwrap())
        .collect()
}

/// Fast check that the records layout matches what each build writes.
#[test]
fn layouts_match_contract_writes() {
    for v in [V1, V2H16, V2H8] {
        for which in ["map", "lookup"] {
            let mut sim = world(v);
            populate(&mut sim, v, which, 200);
        }
    }
    let mut sim = world(V2H16X);
    populate(&mut sim, V2H16X, "accounts", 200);
    let mut sim = world(V2H16Z);
    populate(&mut sim, V2H16Z, "units", 200);
}

#[test]
#[ignore = "slow (100k-key tries); run with --nocapture"]
fn gas_report() {
    println!("\nother accounts holding contract data: {}", others());
    // 1 + 3: map ops at each size, all variants.
    println!("\n### Insert / update / remove, Map<AccountId, u128> vs LookupMap<AccountId, u128>\n");
    println!("{HEADER}");
    for n in sizes() {
        for which in ["map", "lookup"] {
            for v in [V1, V2H16, V2H8] {
                let start = std::time::Instant::now();
                for (op, cs) in ops(v, which, n) {
                    println!("{}", row(&v.label(which), n, op, &cs));
                }
                eprintln!("{v:?} {which} {n}: {:?}", start.elapsed());
            }
        }
    }

    // 2: removal beside a large neighbor.
    println!("\n### Remove a 16-byte non-last entry; the last entry (moved into the gap) holds 1 B or 100 KiB\n");
    println!("{HEADER}");
    for v in [V1, V2H16, V2H8] {
        for nb in [1u32, 100 * 1024] {
            let c = remove_beside(v, nb);
            println!("{}", row(&v.label("blobs"), 2, &format!("remove, neighbor {nb} B"), &[c]));
        }
    }

    // Oct 18 rows (ext wasm).
    println!("\n### Oct 18 rows: page, try_insert of a collection-owning record, remove_step (v2 h16, ext wasm)\n");
    println!("{HEADER}");
    for n in sizes() {
        let start = std::time::Instant::now();
        for (op, cs) in ext_ops(n) {
            if cs.is_empty() {
                println!("| v2 (h16, ext) | {n} | {op} | | | | | | | | | |");
            } else {
                println!("{}", row("v2 (h16, ext)", n, &op, &cs));
            }
        }
        eprintln!("ext {n}: {:?}", start.elapsed());
    }

    // 4: root read and nested read.
    println!("\n### Per-call root cost and the nested read\n");
    println!("{HEADER}");
    for v in [V1, V2H16, V2H8] {
        let mut sim = world(v);
        let label = if v == V1 { "v1 Borsh STATE".to_string() } else { format!("v2 root ({})", if v == V2H8 { "h8" } else { "h16" }) };
        for m in ["ping", "noop_view", "noop_mut"] {
            let c = cost(&call(&mut sim, m, json!({})));
            println!("{}", row(&label, 0, m, &[c]));
        }
        if v != V1 {
            call(&mut sim, "deposit", json!({"token": "usdt", "amount": "5"})).assert_success();
            let o = call(&mut sim, "balance_of", json!({"id": ALICE, "token": "usdt"}));
            assert_eq!(o.json::<String>(), "5");
            let c = cost(&o);
            println!("{}", row(&label, 1, "balance_of (2 hops)", &[c]));
            eprintln!("{v:?} balance_of SHA256_BASE gas {} (= {} calls at 4.540993500 Ggas)", c.sha_base, c.sha_base as f64 / 4_540_993_500.0);
        }
    }
}

/// Compact rows for bisecting a gas change across SDK commits: v2 h16 only, 1k keys.
/// `GAS_RES_DIR=<dir with v2_h16.wasm> cargo test --test storage_v2_gas bisect_rows -- --include-ignored --nocapture`
#[test]
#[ignore = "bisection helper; needs GAS_RES_DIR"]
fn bisect_rows() {
    let v = V2H16;
    let mut line = vec![];
    for which in ["map", "lookup"] {
        for (op, cs) in ops(v, which, 1000) {
            let mut c = cs.clone();
            c.sort_by_key(|c| c.total);
            let m = c[c.len() / 2];
            line.push(format!("{which} {op}: {} (wasm {} load {} bytes {} ops {}/{}/{}/{})", t(m.total), gg(m.wasm), gg(m.load), gg(m.bytes), m.reads, m.writes, m.removes, m.has_keys));
        }
    }
    let c = remove_beside(v, 1);
    line.push(format!("blob remove: {} (wasm {})", t(c.total), gg(c.wasm)));
    let mut sim = world(v);
    for m in ["ping", "noop_view", "noop_mut"] {
        let c = cost(&call(&mut sim, m, json!({})));
        line.push(format!("{m}: {} (wasm {})", t(c.total), gg(c.wasm)));
    }
    call(&mut sim, "deposit", json!({"token": "usdt", "amount": "5"})).assert_success();
    let c = cost(&call(&mut sim, "balance_of", json!({"id": ALICE, "token": "usdt"})));
    line.push(format!("balance_of: {} (wasm {})", t(c.total), gg(c.wasm)));
    println!("BISECT {}", line.join(" | "));
}

/// The six write-path rows (v2 h16 `Map` and `LookupMap` insert/update/remove) at `GAS_SIZES`
/// (default 10k), with the same wasm's `ping` for the op-only figure. Used for the Oct 18
/// flush-regression fix. `GAS_RES_DIR` selects the wasm directory as in `bisect_rows`.
#[test]
#[ignore = "flush-perf helper; run with --nocapture"]
fn perf_rows() {
    let v = V2H16;
    let size = std::fs::metadata(
        std::env::var("GAS_RES_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../res"))
            .join("v2_h16.wasm"),
    )
    .map(|m| m.len())
    .unwrap_or(0);
    let mut sim = world(v);
    let ping = cost(&call(&mut sim, "ping", json!({})));
    println!("\nv2_h16.wasm {size} B; ping {} Tgas (load {})\n", t(ping.total), gg(ping.load));
    println!("{HEADER}");
    for n in std::env::var("GAS_SIZES").map(|_| sizes()).unwrap_or_else(|_| vec![10_000]) {
        for which in ["map", "lookup"] {
            for (op, cs) in ops(v, which, n) {
                println!("{}", row(&v.label(which), n, op, &cs));
                let mut c = cs.clone();
                c.sort_by_key(|c| c.total);
                let m = c[c.len() / 2];
                println!("PERF {which} {op} n={n}: total {} op-only {} wasm {}", t(m.total), gg(m.total - ping.total), gg(m.wasm));
            }
        }
    }
}
