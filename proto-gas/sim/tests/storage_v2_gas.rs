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
}
use Variant::*;

impl Variant {
    fn wasm(self) -> Vec<u8> {
        let name = match self {
            V1 => "v1",
            V2H16 => "v2_h16",
            V2H8 => "v2_h8",
        };
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../res/{name}.wasm"));
        std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e} (run proto-gas/build.sh)", p.display()))
    }
    fn label(self, which: &str) -> String {
        match (self, which) {
            (V1, "map") => "v1 store::IterableMap".into(),
            (V1, "lookup") => "v1 store::LookupMap".into(),
            (V1, "blobs") => "v1 store::IterableMap".into(),
            (V2H16, w) => format!("v2 {} (h16)", if w == "lookup" { "LookupMap" } else { "Map" }),
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
            _ => unreachable!(),
        }
    }
    if v != V1 && which == "map" {
        out.push((vec![V2_ROOT, 3, 3], len.to_le_bytes().to_vec()));
    }
    out
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
    let getter = if which == "map" { "map_get" } else { "lookup_get" };
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
