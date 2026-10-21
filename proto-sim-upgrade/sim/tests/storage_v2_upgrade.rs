//! Storage v2 multi-call features on nearcore's runtime: contract upgrades with schema
//! evolution, `Migrating<V1, V2>` with `migrate_batch`, and `remove_step` clearing across calls,
//! under explored gas, block gaps, overlapping calls and forced `OutOfGas` at self-calls.
//!
//! The contract (`../contract`) comes in five wasms (`../res`, built by `../build.sh`): `v1`
//! (v1 Borsh root), `v2a` (`Migrating` + groups), `v2b` (Group schema changes, tag 4 re-used on
//! purpose), `v2c` (adds a `required` tag, forbidden), `v3` (the wrapper off).
//!
//! How the model works: every successful mutating receipt logs `ev v=<version> seq=<n>`, and
//! `seq` is a root counter carried through every upgrade. The checks fold *every* receipt of
//! every transaction so far (setup included, self-call continuations included) in `seq`
//! order through a model of the SDK's documented behaviour, at the tag level for records, then
//! compare the model with the chain: views through whichever code is deployed, and raw
//! `view_state` probes (keys computed here from the spec, independently of the SDK).
//!
//! A receipt that fails keeps no state change (nearcore's atomicity, not the SDK's), so a call
//! that runs out of gas in the middle of `migrate_batch` or `remove_step` is trivially safe;
//! what this harness can judge that a single-call mock cannot is the cross-receipt behaviour:
//! a `drain`/`clear_loop` continuation starved or landing after a redeploy, a deploy between two
//! calls of a clearing sequence, two calls in one block, and storage as the runtime stakes it.

use near_contract_sim::explore::{Fail, Faults, Harness, Known, Op, OpDone, Plan, Skip, plans};
use near_contract_sim::nearcore::near_primitives::hash::hash as sha256;
use near_contract_sim::{Balance, Failure, Sim};
use proptest::prelude::*;
use proptest::strategy::Union;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const APP: &str = "app.near";
const ROOT: u8 = 0xF0;
const SEED: u32 = 12;

// ---------------------------------------------------------------------------------------------
// versions and wasms
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
enum Ver {
    V1,
    V2a,
    V2b,
    V2c,
    V3,
}

impl Ver {
    const ALL: [Ver; 5] = [Ver::V1, Ver::V2a, Ver::V2b, Ver::V2c, Ver::V3];
    fn name(self) -> &'static str {
        match self {
            Ver::V1 => "v1",
            Ver::V2a => "v2a",
            Ver::V2b => "v2b",
            Ver::V2c => "v2c",
            Ver::V3 => "v3",
        }
    }
    fn parse(s: &str) -> Option<Ver> {
        Ver::ALL.into_iter().find(|v| v.name() == s)
    }
    fn wasm(self) -> &'static [u8] {
        static W: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
        let all = W.get_or_init(|| {
            Ver::ALL
                .iter()
                .map(|v| {
                    let p = format!("{}/../res/{}.wasm", env!("CARGO_MANIFEST_DIR"), v.name());
                    std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}; run ../build.sh"))
                })
                .collect()
        });
        &all[self as usize]
    }
    /// Has the `Migrating` wrapper (and `migrate`, `migrate_batch`, `drain`).
    fn migrating(self) -> bool {
        matches!(self, Ver::V2a | Ver::V2b | Ver::V2c)
    }
    fn is_v2(self) -> bool {
        self != Ver::V1
    }
    /// Group schema B (v2b, v2c, v3) has level/label/flags and tag 4 = claimable.
    fn schema_b(self) -> bool {
        matches!(self, Ver::V2b | Ver::V2c | Ver::V3)
    }
}

fn deployed(sim: &Sim) -> Option<Ver> {
    let r = sim.view(APP, "version", json!({}));
    let bytes = r.result.ok()?;
    let s: String = serde_json::from_slice(&bytes).ok()?;
    Ver::parse(&s)
}

// ---------------------------------------------------------------------------------------------
// keys, computed from the spec (not through the SDK)
// ---------------------------------------------------------------------------------------------

fn borsh_str(s: &str) -> Vec<u8> {
    let mut v = (s.len() as u32).to_le_bytes().to_vec();
    v.extend(s.as_bytes());
    v
}
fn h16(b: &[u8]) -> Vec<u8> {
    sha256(b).as_ref()[..16].to_vec()
}
fn entry_seg(k: &str) -> Vec<u8> {
    let b = borsh_str(k);
    let mut v = h16(&b);
    v.extend(b);
    v
}
/// v2 `items` entry: `F0 01 00 h16(k) borsh(k)`.
fn item_v2(k: &str) -> Vec<u8> {
    let mut v = vec![ROOT, 1, 0];
    v.extend(entry_seg(k));
    v
}
/// v1 `IterableMap` value: `sha256("im" ++ borsh(k))`.
fn item_v1(k: &str) -> Vec<u8> {
    let mut b = b"im".to_vec();
    b.extend(borsh_str(k));
    sha256(&b).as_ref().to_vec()
}
fn v1_slot(i: u32) -> Vec<u8> {
    let mut v = b"iv".to_vec();
    v.extend(i.to_le_bytes());
    v
}
fn group_rec(g: &str) -> Vec<u8> {
    let mut v = vec![ROOT, 2, 0];
    v.extend(entry_seg(g));
    v
}
fn group_flag(g: &str) -> Vec<u8> {
    let mut v = group_rec(g);
    v.push(0);
    v
}
fn coll_entry(g: &str, tag: u8, k: &str) -> Vec<u8> {
    let mut v = group_rec(g);
    v.extend([tag, 0]);
    v.extend(entry_seg(k));
    v
}

fn raw(sim: &Sim, key: &[u8]) -> Option<Vec<u8>> {
    sim.state(APP, key)
}

// ---------------------------------------------------------------------------------------------
// the universe a plan draws from
// ---------------------------------------------------------------------------------------------

fn item_key(i: u8) -> String {
    let i = i as u32 % (6 + SEED);
    if i < 6 { format!("k{i}") } else { format!("s{:02}", i - 6) }
}
fn all_item_keys() -> Vec<String> {
    (0..(6 + SEED) as u8).map(item_key).collect()
}
const GROUPS: u8 = 3;
const MEMBERS: u8 = 6;
const FLAGS: u8 = 3;
fn gname(g: u8) -> String {
    format!("g{}", g % GROUPS)
}
fn mname(m: u8) -> String {
    format!("m{}", m % MEMBERS)
}
fn fname(f: u8) -> String {
    format!("f{}", f % FLAGS)
}

// ---------------------------------------------------------------------------------------------
// ops
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
enum UpOp {
    Insert { k: u8, v: u64 },
    Remove { k: u8 },
    MigrateBatch { n: u32 },
    /// `migrate_batch(n)` then self-calls, up to `rounds` receipts.
    Drain { n: u32, rounds: u32 },
    CreateGroup { g: u8, weight: u64, x: u64 },
    SetWeight { g: u8, w: u64 },
    /// v2a: `locked`; v2b on: `claimable` (the re-used tag 4).
    SetField4 { g: u8, x: u64 },
    SetLevel { g: u8, x: u64 },
    SetLabel { g: u8, x: u64 },
    AddMember { g: u8, m: u8, v: u64 },
    RemoveMember { g: u8, m: u8 },
    AddFlag { g: u8, f: u8 },
    RemoveStep { g: u8, n: u32 },
    /// `remove_step(g, n)` then self-calls, up to `rounds` receipts.
    ClearLoop { g: u8, n: u32, rounds: u32 },
    RemoveGroup { g: u8 },
    /// Redeploys mid-plan. From v1, `migrate` runs in the same transaction iff `migrate`.
    Upgrade { to: Ver, migrate: bool },
    /// `migrate` on its own (the deploy-without-migrate window).
    Migrate,
}

fn call(sim: &mut Sim, method: &str, args: Value) {
    sim.call(APP, APP, method).args_json(args).submit();
}

impl Op for UpOp {
    fn submit(&self, sim: &mut Sim) -> Result<(), Skip> {
        // The version now. Under overlap a deploy may still be pending; a call that lands on
        // other code then fails or runs there, and the model follows what ran.
        let cur = deployed(sim).ok_or("no version view")?;
        let v2 = || -> Result<(), Skip> { if cur.is_v2() { Ok(()) } else { Err("v1 has no groups".into()) } };
        let b = || -> Result<(), Skip> { if cur.schema_b() { Ok(()) } else { Err("not on schema B".into()) } };
        match *self {
            UpOp::Insert { k, v } => call(sim, "insert", json!({ "k": item_key(k), "v": v })),
            UpOp::Remove { k } => call(sim, "remove", json!({ "k": item_key(k) })),
            UpOp::MigrateBatch { n } => {
                if !cur.migrating() {
                    return Err("no Migrating".into());
                }
                call(sim, "migrate_batch", json!({ "n": n }))
            }
            UpOp::Drain { n, rounds } => {
                if !cur.migrating() {
                    return Err("no Migrating".into());
                }
                call(sim, "drain", json!({ "n": n, "rounds": rounds }))
            }
            UpOp::CreateGroup { g, weight, x } => {
                v2()?;
                call(sim, "create_group", json!({ "g": gname(g), "weight": weight, "x": x }))
            }
            UpOp::SetWeight { g, w } => {
                v2()?;
                call(sim, "set_weight", json!({ "g": gname(g), "w": w }))
            }
            UpOp::SetField4 { g, x } => {
                v2()?;
                call(sim, "set_field4", json!({ "g": gname(g), "x": x }))
            }
            UpOp::SetLevel { g, x } => {
                b()?;
                call(sim, "set_level", json!({ "g": gname(g), "x": x }))
            }
            UpOp::SetLabel { g, x } => {
                b()?;
                call(sim, "set_label", json!({ "g": gname(g), "x": x }))
            }
            UpOp::AddMember { g, m, v } => {
                v2()?;
                call(sim, "add_member", json!({ "g": gname(g), "m": mname(m), "v": v }))
            }
            UpOp::RemoveMember { g, m } => {
                v2()?;
                call(sim, "remove_member", json!({ "g": gname(g), "m": mname(m) }))
            }
            UpOp::AddFlag { g, f } => {
                b()?;
                call(sim, "add_flag", json!({ "g": gname(g), "f": fname(f) }))
            }
            UpOp::RemoveStep { g, n } => {
                v2()?;
                call(sim, "remove_step", json!({ "g": gname(g), "n": n }))
            }
            UpOp::ClearLoop { g, n, rounds } => {
                v2()?;
                call(sim, "clear_loop", json!({ "g": gname(g), "n": n, "rounds": rounds }))
            }
            UpOp::RemoveGroup { g } => {
                v2()?;
                call(sim, "remove_group", json!({ "g": gname(g) }))
            }
            UpOp::Upgrade { to, migrate } => {
                if to == cur || to == Ver::V1 {
                    return Err("same version, or back to v1".into());
                }
                if to == Ver::V3 {
                    // The documented precondition: the wrapper comes off once the old side is
                    // empty. The old side only grows under v1, so this cannot race.
                    let empty = cur.migrating()
                        && sim.view(APP, "old_is_empty", json!({})).result.ok().as_deref() == Some(b"true");
                    if !empty {
                        return Err("v3 only after the old side is empty".into());
                    }
                }
                if cur == Ver::V1 && migrate {
                    sim.tx(APP, APP).deploy(to.wasm()).call("migrate").submit();
                } else {
                    sim.tx(APP, APP).deploy(to.wasm()).submit();
                }
            }
            UpOp::Migrate => {
                if !cur.migrating() {
                    return Err("no migrate in this version".into());
                }
                call(sim, "migrate", json!({}))
            }
        }
        Ok(())
    }
}

fn ops(upgrade_weight: u32) -> impl Strategy<Value = UpOp> {
    let g = || 0..GROUPS;
    let ver = prop_oneof![Just(Ver::V2a), Just(Ver::V2b), Just(Ver::V2c), Just(Ver::V3)];
    Union::new_weighted(vec![
        (3, (any::<u8>(), 0..1000u64).prop_map(|(k, v)| UpOp::Insert { k, v }).boxed()),
        (2, any::<u8>().prop_map(|k| UpOp::Remove { k }).boxed()),
        (2, (1..=20u32).prop_map(|n| UpOp::MigrateBatch { n }).boxed()),
        (2, (1..=6u32, 1..=5u32).prop_map(|(n, rounds)| UpOp::Drain { n, rounds }).boxed()),
        (3, (g(), 0..100u64, 0..50u64).prop_map(|(g, weight, x)| UpOp::CreateGroup { g, weight, x }).boxed()),
        (2, (g(), 0..100u64).prop_map(|(g, w)| UpOp::SetWeight { g, w }).boxed()),
        (2, (g(), 1..50u64).prop_map(|(g, x)| UpOp::SetField4 { g, x }).boxed()),
        (1, (g(), 0..50u64).prop_map(|(g, x)| UpOp::SetLevel { g, x }).boxed()),
        (1, (g(), 0..50u64).prop_map(|(g, x)| UpOp::SetLabel { g, x }).boxed()),
        (4, (g(), 0..MEMBERS, 0..1000u64).prop_map(|(g, m, v)| UpOp::AddMember { g, m, v }).boxed()),
        (1, (g(), 0..MEMBERS).prop_map(|(g, m)| UpOp::RemoveMember { g, m }).boxed()),
        (1, (g(), 0..FLAGS).prop_map(|(g, f)| UpOp::AddFlag { g, f }).boxed()),
        (3, (g(), 1..=12u32).prop_map(|(g, n)| UpOp::RemoveStep { g, n }).boxed()),
        (2, (g(), 3..=8u32, 1..=6u32).prop_map(|(g, n, rounds)| UpOp::ClearLoop { g, n, rounds }).boxed()),
        (1, g().prop_map(|g| UpOp::RemoveGroup { g }).boxed()),
        (upgrade_weight, (ver, any::<bool>()).prop_map(|(to, migrate)| UpOp::Upgrade { to, migrate }).boxed()),
        (1, Just(UpOp::Migrate).boxed()),
    ])
}

// ---------------------------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------------------------

/// Storage the account stakes beyond its contract data and code (account record, access key).
static BASE_BYTES: OnceLock<u64> = OnceLock::new();

fn state_bytes(sim: &Sim) -> u64 {
    sim.view_state(APP, &[]).iter().map(|(k, v)| (k.len() + v.len() + 40) as u64).sum()
}

/// v1 deployed, `new`, and `seed(12)`: the old side holds `s00..s11`.
fn v1_world() -> Sim {
    let mut sim = Sim::new();
    sim.create_account(APP, Balance::from_near(100));
    sim.deploy(APP, Ver::V1.wasm().to_vec());
    sim.call(APP, APP, "new").transact().assert_success();
    sim.call(APP, APP, "seed").args_json(json!({ "n": SEED })).transact().assert_success();
    let usage = sim.account(APP).unwrap().storage_usage;
    BASE_BYTES.get_or_init(|| usage - Ver::V1.wasm().len() as u64 - state_bytes(&sim));
    sim
}

/// v2a deployed and migrated in one transaction; two groups with members; the old side still
/// holds all 12 seeded items.
fn v2a_world() -> Sim {
    let mut sim = v1_world();
    sim.tx(APP, APP).deploy(Ver::V2a.wasm()).call("migrate").transact().assert_success();
    for (g, w, x) in [("g0", 10, 5), ("g1", 20, 6)] {
        sim.call(APP, APP, "create_group").args_json(json!({ "g": g, "weight": w, "x": x })).transact().assert_success();
    }
    for m in ["m0", "m1", "m2"] {
        sim.call(APP, APP, "add_member").args_json(json!({ "g": "g0", "m": m, "v": 7 })).transact().assert_success();
    }
    sim
}

/// v2a's world upgraded to v2b: g0 rewritten by schema B (tag 3 dropped, level/label and the
/// flags marker written, tag 4 still v2a's `locked`), g2 created by v2b with a flag. Downgrades
/// to v2a and A3 are one op away.
fn v2b_world() -> Sim {
    let mut sim = v2a_world();
    sim.tx(APP, APP).deploy(Ver::V2b.wasm()).transact().assert_success();
    sim.call(APP, APP, "set_level").args_json(json!({ "g": "g0", "x": 4 })).transact().assert_success();
    sim.call(APP, APP, "create_group").args_json(json!({ "g": "g2", "weight": 1, "x": 2 })).transact().assert_success();
    sim.call(APP, APP, "add_flag").args_json(json!({ "g": "g2", "f": "f0" })).transact().assert_success();
    sim.call(APP, APP, "add_member").args_json(json!({ "g": "g2", "m": "m3", "v": 1 })).transact().assert_success();
    sim
}

// ---------------------------------------------------------------------------------------------
// the model
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Old,
    New,
}

/// Who wrote tag 4: v2a's `locked` or schema B's `claimable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prov {
    Locked,
    Claimable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TagV {
    Str(String),
    U64(u64),
    U32(u32),
    Marker(u8),
}

impl TagV {
    fn wire_body(&self) -> (u8, Vec<u8>) {
        match self {
            TagV::Str(s) => (0, borsh_str(s)),
            TagV::U64(x) => (0, x.to_le_bytes().to_vec()),
            TagV::U32(x) => (0, x.to_le_bytes().to_vec()),
            TagV::Marker(k) => (1, vec![*k]),
        }
    }
}

const MAP_KIND: u8 = 0x81;
const SET_KIND: u8 = 0x83;

fn varint(out: &mut Vec<u8>, mut x: u64) {
    loop {
        let b = (x & 0x7f) as u8;
        x >>= 7;
        if x == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

/// The record format of the spec: per tag ascending, `varint(tag << 2 | wire) varint(len) body`.
fn encode_record(tags: &BTreeMap<u32, TagV>) -> Vec<u8> {
    let mut out = Vec::new();
    for (t, v) in tags {
        let (wire, body) = v.wire_body();
        varint(&mut out, ((*t as u64) << 2) | wire as u64);
        varint(&mut out, body.len() as u64);
        out.extend(body);
    }
    out
}

/// A group record as the version decodes it (`None` field = not part of that schema).
#[derive(Clone, Debug, Default, PartialEq)]
struct Decoded {
    owner: String,
    weight: u64,
    note: Option<String>,
    f4: u64,
    level: Option<u32>,
    label: Option<String>,
    admin: Option<String>,
}

#[derive(Clone, Debug)]
struct GroupM {
    tags: BTreeMap<u32, TagV>,
    /// Provenance of tag 4's bytes.
    prov4: Prov,
    /// `members` key vector, in slot order (Map: push, swap-remove, clearing pops the end).
    members: Vec<(String, u64)>,
    members_hdr: bool,
    flags: Vec<String>,
    flags_hdr: bool,
    clearing: bool,
    /// Flags this record inherited from a removed predecessor at the same key (A3).
    inherited: bool,
}

#[derive(Clone, Debug, Default)]
struct Model {
    seq: u64,
    migrated: bool,
    /// The v1 `IterableMap` key vector (the old side), in slot order.
    v1_order: Vec<String>,
    items: BTreeMap<String, (u64, Side)>,
    new_len: u32,
    new_rev: bool,
    groups: BTreeMap<String, GroupM>,
    /// Flag entries (and whether their header) a removal through v2a left behind: the record
    /// carried no tag-8 marker, so the A3 check could not see them.
    orphans: BTreeMap<String, (Vec<String>, bool)>,
    /// What the model saw that should not have happened: (kind, message).
    violations: Vec<(String, String)>,
}

fn str_of(t: &BTreeMap<u32, TagV>, tag: u32) -> Option<String> {
    match t.get(&tag) {
        Some(TagV::Str(s)) => Some(s.clone()),
        _ => None,
    }
}
fn u64_of(t: &BTreeMap<u32, TagV>, tag: u32) -> Option<u64> {
    match t.get(&tag) {
        Some(TagV::U64(x)) => Some(*x),
        _ => None,
    }
}
fn u32_of(t: &BTreeMap<u32, TagV>, tag: u32) -> Option<u32> {
    match t.get(&tag) {
        Some(TagV::U32(x)) => Some(*x),
        _ => None,
    }
}

/// Tags each version declares inline (scalars and collection markers), and its reserved tags.
fn declared(v: Ver) -> (&'static [u32], &'static [u32]) {
    match v {
        Ver::V2a => (&[1, 2, 3, 4, 5], &[]),
        Ver::V2b | Ver::V3 => (&[1, 2, 4, 5, 6, 7, 8], &[3]),
        Ver::V2c => (&[1, 2, 4, 5, 6, 7, 8, 9], &[3]),
        Ver::V1 => (&[], &[]),
    }
}

impl GroupM {
    /// Decoding through `v`; `Err` where the SDK must refuse (a missing `required` tag).
    fn decode(&self, v: Ver) -> Result<Decoded, String> {
        let t = &self.tags;
        let mut d = Decoded {
            owner: str_of(t, 1).unwrap_or_default(),
            weight: u64_of(t, 2).unwrap_or(0),
            f4: u64_of(t, 4).unwrap_or(0),
            ..Default::default()
        };
        if v == Ver::V2a {
            d.note = Some(str_of(t, 3).unwrap_or_default());
        } else {
            d.level = Some(u32_of(t, 6).unwrap_or(7));
            d.label = Some(str_of(t, 7).unwrap_or_else(|| "unlabeled".into()));
        }
        if v == Ver::V2c {
            d.admin = Some(str_of(t, 9).ok_or("missing required tag 9")?);
        }
        Ok(d)
    }

    /// What `v`'s encoder writes for `d` over the stored bytes: its declared fields, plus the
    /// stored tags it does not know and does not reserve.
    fn encode_from(&mut self, v: Ver, d: &Decoded) {
        let (decl, reserved) = declared(v);
        let mut out = BTreeMap::new();
        out.insert(1, TagV::Str(d.owner.clone()));
        out.insert(2, TagV::U64(d.weight));
        out.insert(4, TagV::U64(d.f4));
        out.insert(5, TagV::Marker(MAP_KIND));
        if v == Ver::V2a {
            out.insert(3, TagV::Str(d.note.clone().unwrap()));
        } else {
            out.insert(6, TagV::U32(d.level.unwrap()));
            out.insert(7, TagV::Str(d.label.clone().unwrap()));
            out.insert(8, TagV::Marker(SET_KIND));
        }
        if v == Ver::V2c {
            out.insert(9, TagV::Str(d.admin.clone().unwrap()));
        }
        for (t, val) in &self.tags {
            if !decl.contains(t) && !reserved.contains(t) {
                out.insert(*t, val.clone());
            }
        }
        self.tags = out;
    }

    /// The spec's budget rule (host removes) for one `remove_step(n)` call through `v`. Applies
    /// the progress; returns whether the record is gone.
    fn step(&mut self, v: Ver, n: u32) -> bool {
        let pend = if self.clearing { 0 } else { 1 };
        let mut budget = n - pend;
        let mut ok = true;
        // Field order: members (tag 5), then flags (tag 8) where the version declares it.
        let mut cols: Vec<u8> = vec![5];
        if v.schema_b() {
            cols.push(8);
        }
        'cols: for c in cols {
            loop {
                let len = if c == 5 { self.members.len() } else { self.flags.len() };
                if len == 0 {
                    let hdr = if c == 5 { &mut self.members_hdr } else { &mut self.flags_hdr };
                    let cost = u32::from(*hdr);
                    if cost > budget {
                        ok = false;
                        break 'cols;
                    }
                    budget -= cost;
                    *hdr = false;
                    break;
                }
                if budget < 3 {
                    ok = false;
                    break 'cols;
                }
                budget -= 3;
                if c == 5 {
                    self.members.pop();
                    self.members_hdr = true;
                } else {
                    self.flags.pop();
                    self.flags_hdr = true;
                }
            }
        }
        if ok && 3 <= budget + pend {
            true
        } else {
            self.clearing = true;
            false
        }
    }
}

struct Event {
    seq: u64,
    ver: Ver,
    method: String,
    args: Value,
    ret: Option<Vec<u8>>,
}

/// Every successful receipt on the app that logged an event, from every transaction so far.
fn events(sim: &Sim) -> Vec<Event> {
    let mut out = Vec::new();
    for h in sim.txs() {
        let o = sim.outcome(h);
        for r in &o.receipts {
            if r.executor_id.as_str() != APP || !r.is_success() {
                continue;
            }
            let Some(ev) = r.logs.iter().find_map(|l| l.strip_prefix("ev ")) else { continue };
            let mut ver = None;
            let mut seq = None;
            for kv in ev.split(' ') {
                if let Some(v) = kv.strip_prefix("v=") {
                    ver = Ver::parse(v);
                }
                if let Some(s) = kv.strip_prefix("seq=") {
                    seq = s.parse().ok();
                }
            }
            let method = r.method_names.last().cloned().unwrap_or_default();
            let args = r.args.last().and_then(|a| serde_json::from_slice(a).ok()).unwrap_or(Value::Null);
            out.push(Event { seq: seq.unwrap(), ver: ver.unwrap(), method, args, ret: r.value().map(<[u8]>::to_vec) });
        }
    }
    out.sort_by_key(|e| e.seq);
    out
}

fn s(a: &Value, f: &str) -> String {
    a[f].as_str().unwrap_or_default().to_string()
}
fn n(a: &Value, f: &str) -> u64 {
    a[f].as_u64().unwrap_or(0)
}

impl Model {
    fn bad(&mut self, kind: &str, msg: String) {
        self.violations.push((kind.into(), msg));
    }

    /// Removes a group as `v` does: a version without the flags field leaves them behind.
    fn drop_group(&mut self, g: &str, v: Ver) -> bool {
        let Some(gm) = self.groups.remove(g) else { return false };
        if !v.schema_b() && (!gm.flags.is_empty() || gm.flags_hdr) {
            self.orphans.insert(g.to_string(), (gm.flags, gm.flags_hdr));
        }
        true
    }

    fn v1_remove(&mut self, k: &str) {
        if let Some(i) = self.v1_order.iter().position(|x| x == k) {
            self.v1_order.swap_remove(i);
        }
    }

    fn new_insert(&mut self, k: &str, v: u64) {
        let fresh = !matches!(self.items.get(k), Some((_, Side::New)));
        if fresh {
            self.new_len += 1;
        }
        self.items.insert(k.into(), (v, Side::New));
    }

    fn new_remove(&mut self, k: &str) {
        if let Some((_, Side::New)) = self.items.get(k) {
            self.items.remove(k);
            self.new_len -= 1;
            self.new_rev = true;
        }
    }

    /// The group, if a write below it may proceed; records a violation otherwise.
    fn writable(&mut self, e: &Event, g: &str) -> bool {
        match self.groups.get(g) {
            None => {
                self.bad("write to an absent group succeeded", format!("{} on {g} (seq {})", e.method, e.seq));
                false
            }
            Some(gm) if gm.clearing => {
                self.bad("write below a clearing record succeeded", format!("{} on {g} (seq {})", e.method, e.seq));
                false
            }
            Some(_) => true,
        }
    }

    fn apply(&mut self, e: &Event) {
        let a = &e.args;
        let v = e.ver;
        match e.method.as_str() {
            "new" => {}
            "seed" => {
                for i in 0..n(a, "n") {
                    let k = format!("s{i:02}");
                    if !self.items.contains_key(&k) {
                        self.v1_order.push(k.clone());
                    }
                    self.items.insert(k, (1000 + i, Side::Old));
                }
            }
            "migrate" => {
                if self.migrated {
                    self.bad("migrate ran twice", format!("seq {}", e.seq));
                }
                self.migrated = true;
            }
            "insert" => {
                let (k, val) = (s(a, "k"), n(a, "v"));
                if v == Ver::V1 {
                    if !self.items.contains_key(&k) {
                        self.v1_order.push(k.clone());
                    }
                    self.items.insert(k, (val, Side::Old));
                } else {
                    if let Some((_, Side::Old)) = self.items.get(&k) {
                        self.v1_remove(&k);
                    }
                    self.new_insert(&k, val);
                }
            }
            "remove" => {
                let k = s(a, "k");
                let want = self.items.get(&k).map(|x| x.0);
                let got: Option<u64> = e.ret.as_deref().and_then(|b| serde_json::from_slice(b).ok()).flatten();
                if want != got {
                    self.bad("remove returned another value", format!("remove({k}) returned {got:?}, model {want:?}"));
                }
                match self.items.get(&k) {
                    Some((_, Side::Old)) => {
                        self.v1_remove(&k);
                        self.items.remove(&k);
                    }
                    Some((_, Side::New)) => self.new_remove(&k),
                    None => {}
                }
            }
            "migrate_batch" | "drain" => {
                let take: Vec<String> = self.v1_order.iter().take(n(a, "n") as usize).cloned().collect();
                for k in &take {
                    self.v1_remove(k);
                    let val = self.items[k].0;
                    self.new_insert(k, val);
                }
                let got: Option<u32> = e.ret.as_deref().and_then(|b| serde_json::from_slice(b).ok());
                if got != Some(take.len() as u32) {
                    self.bad("migrate_batch moved another count", format!("{}({}) returned {got:?}, model {}", e.method, n(a, "n"), take.len()));
                }
            }
            "create_group" => {
                let g = s(a, "g");
                if self.groups.contains_key(&g) {
                    self.bad("create over an existing group succeeded", format!("{g} (seq {})", e.seq));
                    return;
                }
                let (w, x) = (n(a, "weight"), n(a, "x"));
                let mut gm = GroupM {
                    tags: BTreeMap::new(),
                    prov4: if v == Ver::V2a { Prov::Locked } else { Prov::Claimable },
                    members: vec![],
                    members_hdr: false,
                    flags: vec![],
                    flags_hdr: false,
                    clearing: false,
                    inherited: false,
                };
                if let Some((flags, hdr)) = self.orphans.remove(&g) {
                    gm.flags = flags;
                    gm.flags_hdr = hdr;
                    gm.inherited = true;
                }
                let d = if v == Ver::V2a {
                    Decoded { owner: "o".into(), weight: w, note: Some(format!("n{x}")), f4: x, ..Default::default() }
                } else {
                    Decoded {
                        owner: "o".into(),
                        weight: w,
                        f4: 0,
                        level: Some(x as u32),
                        label: Some(format!("l{x}")),
                        admin: (v == Ver::V2c).then(|| "a".into()),
                        ..Default::default()
                    }
                };
                gm.encode_from(v, &d);
                self.groups.insert(g, gm);
            }
            "set_weight" | "set_field4" | "set_level" | "set_label" => {
                let g = s(a, "g");
                if !self.writable(e, &g) {
                    return;
                }
                let gm = self.groups.get_mut(&g).unwrap();
                let mut d = match gm.decode(v) {
                    Ok(d) => d,
                    Err(why) => {
                        let msg = format!("{} on {g} through {v:?}: {why}", e.method);
                        self.bad("get_mut decoded a record the schema refuses", msg);
                        return;
                    }
                };
                let x = n(a, "x");
                match e.method.as_str() {
                    "set_weight" => d.weight = n(a, "w"),
                    "set_field4" => {
                        d.f4 = x;
                        gm.prov4 = if v == Ver::V2a { Prov::Locked } else { Prov::Claimable };
                    }
                    "set_level" => d.level = Some(x as u32),
                    _ => d.label = Some(format!("L{x}")),
                }
                gm.encode_from(v, &d);
            }
            "add_member" | "remove_member" | "add_flag" => {
                let g = s(a, "g");
                if !self.writable(e, &g) {
                    return;
                }
                let gm = self.groups.get_mut(&g).unwrap();
                match e.method.as_str() {
                    "add_member" => {
                        let (m, val) = (s(a, "m"), n(a, "v"));
                        match gm.members.iter_mut().find(|(k, _)| *k == m) {
                            Some(slot) => slot.1 = val,
                            None => gm.members.push((m, val)),
                        }
                        gm.members_hdr = true;
                    }
                    "remove_member" => {
                        let m = s(a, "m");
                        if let Some(i) = gm.members.iter().position(|(k, _)| *k == m) {
                            gm.members.swap_remove(i);
                        }
                    }
                    _ => {
                        let f = s(a, "f");
                        if !gm.flags.contains(&f) {
                            gm.flags.push(f);
                        }
                        gm.flags_hdr = true;
                    }
                }
            }
            "remove_step" | "clear_loop" => {
                let (g, step) = (s(a, "g"), n(a, "n") as u32);
                let got: Option<bool> = e.ret.as_deref().and_then(|b| serde_json::from_slice(b).ok());
                if step < 3 {
                    self.bad("remove_step below the minimum succeeded", format!("n = {step}"));
                    return;
                }
                let Some(gm) = self.groups.get(&g) else {
                    if got != Some(true) {
                        self.bad("remove_step progress differs from the spec rule", format!("absent {g}: returned {got:?}"));
                    }
                    return;
                };
                let a3 = v == Ver::V2a && gm.tags.contains_key(&8);
                let undecodable = gm.decode(v).err();
                if a3 {
                    self.bad("removal with an unknown collection tag succeeded (A3)", format!("{g} through v2a"));
                }
                if let Some(why) = undecodable {
                    self.bad("get_mut decoded a record the schema refuses", format!("remove_step on {g} through {v:?}: {why}"));
                }
                let done = self.groups.get_mut(&g).unwrap().step(v, step);
                if got != Some(done) {
                    let msg = format!("{}({g}, {step}) through {v:?} returned {got:?}, rule says done = {done}", e.method);
                    self.bad("remove_step progress differs from the spec rule", msg);
                }
                if got == Some(true) {
                    self.drop_group(&g, v);
                } else if let Some(gm) = self.groups.get_mut(&g) {
                    gm.clearing = true;
                }
            }
            "remove_group" => {
                let g = s(a, "g");
                let got: Option<bool> = e.ret.as_deref().and_then(|b| serde_json::from_slice(b).ok());
                let (a3, undecodable) = match self.groups.get(&g) {
                    Some(gm) => (v == Ver::V2a && gm.tags.contains_key(&8), gm.decode(v).err()),
                    None => (false, None),
                };
                if a3 {
                    self.bad("removal with an unknown collection tag succeeded (A3)", format!("{g} through v2a"));
                }
                if let Some(why) = undecodable {
                    self.bad("get_mut decoded a record the schema refuses", format!("remove_group({g}) through {v:?}: {why}"));
                }
                let want = self.drop_group(&g, v);
                if got != Some(want) {
                    self.bad("remove returned another value", format!("remove_group({g}) returned {got:?}, model {want}"));
                }
            }
            other => self.bad("unknown event", other.to_string()),
        }
    }
}

/// The model after every event so far, and whether `seq` ran without a gap.
fn fold(sim: &Sim) -> (Model, Result<(), String>) {
    let evs = events(sim);
    let mut m = Model::default();
    let mut gapless = Ok(());
    for (i, e) in evs.iter().enumerate() {
        if e.seq != i as u64 + 1 && gapless.is_ok() {
            gapless = Err(format!("event {i} has seq {} (methods around: {})", e.seq, e.method));
        }
        m.apply(e);
        m.seq = e.seq;
    }
    (m, gapless)
}

// ---------------------------------------------------------------------------------------------
// checks
// ---------------------------------------------------------------------------------------------

fn view_json(sim: &Sim, method: &str, args: Value) -> Result<Value, Failure> {
    sim.view(APP, method, args).result.map(|b| serde_json::from_slice(&b).unwrap_or(Value::Null))
}

/// The deployed code can read state: v1 with STATE, or a v2 root that exists.
fn readable(m: &Model, d: Ver) -> bool {
    if d == Ver::V1 { !m.migrated } else { m.migrated }
}

fn check_seq(sim: &Sim) -> Result<(), Fail> {
    let (m, gapless) = fold(sim);
    gapless.map_err(Fail::new)?;
    let d = deployed(sim).ok_or("no version")?;
    if readable(&m, d) {
        let got = view_json(sim, "seq", json!({})).map_err(|e| Fail::new(format!("seq view failed: {e}")))?;
        if got.as_u64() != Some(m.seq) {
            return Err(Fail::new(format!("seq view {got}, last event {}", m.seq)));
        }
    }
    Ok(())
}

fn check_violations(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    match m.violations.first() {
        None => Ok(()),
        Some((kind, msg)) => Err(Fail::new(format!("{kind}: {msg}")).key(kind.clone())),
    }
}

fn check_items_read_back(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    let d = deployed(sim).ok_or("no version")?;
    for k in all_item_keys() {
        let r = view_json(sim, "get", json!({ "k": k }));
        if !readable(&m, d) {
            if r.is_ok() {
                return Err(Fail::new(format!("{d:?} read {k} from state it cannot decode")).key("unreadable state read"));
            }
            continue;
        }
        let got = r.map_err(|e| Fail::new(format!("get({k}) through {d:?} failed: {e}")))?;
        let want = m.items.get(&k).map(|x| x.0);
        if got.as_u64() != want {
            return Err(Fail::new(format!("get({k}) through {d:?} = {got}, model {want:?}")).key("item value"));
        }
    }
    Ok(())
}

fn check_one_side(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    for k in all_item_keys() {
        let old = raw(sim, &item_v1(&k));
        let new = raw(sim, &item_v2(&k));
        if old.is_some() && new.is_some() {
            return Err(Fail::new(format!("{k} has a v1 key and a v2 key")).key("both sides"));
        }
        let want = m.items.get(&k).copied();
        let got = match (&old, &new) {
            (Some(b), None) => Some((u64::from_le_bytes(b[..8].try_into().unwrap()), Side::Old)),
            (None, Some(b)) => Some((u64::from_le_bytes(b[..8].try_into().unwrap()), Side::New)),
            _ => None,
        };
        if got != want {
            return Err(Fail::new(format!("{k}: raw {got:?}, model {want:?}")).key("raw side"));
        }
    }
    // The v1 key vector, slot by slot, and nothing past it.
    for (i, k) in m.v1_order.iter().enumerate() {
        if raw(sim, &v1_slot(i as u32)) != Some(borsh_str(k)) {
            return Err(Fail::new(format!("v1 slot {i}: raw {:?}, model {k}", raw(sim, &v1_slot(i as u32)))).key("v1 key vector"));
        }
    }
    let slots = sim.view_state(APP, b"iv").len();
    if slots != m.v1_order.len() {
        return Err(Fail::new(format!("{slots} v1 slots, model {}", m.v1_order.len())).key("v1 key vector"));
    }
    if m.migrated && m.v1_order.is_empty() {
        let stray: Vec<_> = sim.view_state(APP, &[]).into_iter().filter(|(k, _)| k[0] != ROOT).map(|(k, _)| k).collect();
        if !stray.is_empty() {
            return Err(Fail::new(format!("drained, but {} non-v2 keys remain: {:02x?}", stray.len(), &stray[..1])).key("v1 leftovers"));
        }
    }
    Ok(())
}

fn check_group_records(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    for gi in 0..GROUPS {
        let g = gname(gi);
        let rec = raw(sim, &group_rec(&g));
        let flag = raw(sim, &group_flag(&g));
        match m.groups.get(&g) {
            None => {
                let mut left = sim.view_state(APP, &group_rec(&g));
                if let Some((flags, hdr)) = m.orphans.get(&g) {
                    // A `Set` keeps 3 keys per entry (entry, index, key slot) and a header.
                    let at = group_rec(&g).len();
                    let under: Vec<&Vec<u8>> = left.iter().map(|(k, _)| k).filter(|k| k[at] == 8).collect();
                    let kind = |c: u8| under.iter().filter(|k| k.len() > at + 1 && k[at + 1] == c).count();
                    let mut entries: Vec<Vec<u8>> = under.iter().filter(|k| k[at + 1] == 0).map(|k| k.to_vec()).collect();
                    entries.sort();
                    let mut want: Vec<Vec<u8>> = flags.iter().map(|f| coll_entry(&g, 8, f)).collect();
                    want.sort();
                    let n = flags.len();
                    if entries != want || kind(1) != n || kind(2) != n || kind(3) != usize::from(*hdr) {
                        return Err(Fail::new(format!(
                            "{g}: orphaned flag keys: entries {entries:02x?}, index {}, slots {}, header {}; model {flags:?}, header {hdr}",
                            kind(1), kind(2), kind(3)
                        ))
                        .key("orphaned keys"));
                    }
                    left.retain(|(k, _)| k[at] != 8);
                }
                if !left.is_empty() {
                    return Err(Fail::new(format!("{g} is gone, {} keys under it remain: {:02x?}", left.len(), left[0].0)).key("orphaned keys"));
                }
            }
            Some(gm) => {
                let want = encode_record(&gm.tags);
                let (at_key, at_flag) = if gm.clearing { (None, Some(&want)) } else { (Some(&want), None) };
                if rec.as_ref() != at_key || flag.as_ref() != at_flag {
                    return Err(Fail::new(format!(
                        "{g} (clearing {}): record {:02x?}, flag {:02x?}, model {:02x?}",
                        gm.clearing, rec, flag, want
                    ))
                    .key("record bytes"));
                }
                for mi in 0..MEMBERS {
                    let mm = mname(mi);
                    let got = raw(sim, &coll_entry(&g, 5, &mm)).map(|b| u64::from_le_bytes(b[..8].try_into().unwrap()));
                    let want = gm.members.iter().find(|(k, _)| *k == mm).map(|x| x.1);
                    if got != want {
                        return Err(Fail::new(format!("{g}.members[{mm}]: raw {got:?}, model {want:?}")).key("member entries"));
                    }
                }
                for fi in 0..FLAGS {
                    let f = fname(fi);
                    let got = raw(sim, &coll_entry(&g, 8, &f)).is_some();
                    if got != gm.flags.contains(&f) {
                        return Err(Fail::new(format!("{g}.flags[{f}]: raw {got}, model {}", !got)).key("flag entries"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// The intent behind A3: a collection never outlives its record, and a new record at the same
/// key starts empty.
fn check_no_orphans(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    let cause = "A3 hole: a collection written through at_mut under a record without its marker";
    if let Some((g, (flags, _))) = m.orphans.iter().next() {
        return Err(Fail::new(format!("{g} was removed through v2a and left its flags {flags:?} behind")).cause(cause));
    }
    if let Some((g, gm)) = m.groups.iter().find(|(_, gm)| gm.inherited) {
        return Err(Fail::new(format!("new {g} inherited flags {:?} from the removed one", gm.flags)).cause(cause));
    }
    Ok(())
}

fn decoded_json(d: &Decoded, v: Ver) -> Value {
    if v == Ver::V2a {
        json!({ "owner": d.owner, "weight": d.weight, "note": d.note, "locked": d.f4 })
    } else if v == Ver::V2c {
        json!({ "owner": d.owner, "weight": d.weight, "claimable": d.f4, "level": d.level, "label": d.label, "admin": d.admin })
    } else {
        json!({ "owner": d.owner, "weight": d.weight, "claimable": d.f4, "level": d.level, "label": d.label })
    }
}

fn check_group_views(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    let d = deployed(sim).ok_or("no version")?;
    if d == Ver::V1 || !m.migrated {
        return Ok(());
    }
    for gi in 0..GROUPS {
        let g = gname(gi);
        let r = view_json(sim, "group", json!({ "g": g }));
        let want = m.groups.get(&g).map(|gm| gm.decode(d));
        match (r, want) {
            (Ok(Value::Null), None) => {}
            (Ok(got), Some(Ok(dd))) => {
                let want = decoded_json(&dd, d);
                if got != want {
                    return Err(Fail::new(format!("group({g}) through {d:?} = {got}, model {want}")).key("group view"));
                }
            }
            (Err(_), Some(Err(_))) => {}
            (got, want) => {
                return Err(Fail::new(format!("group({g}) through {d:?} = {got:?}, model {want:?}")).key("group view"));
            }
        }
        if let Some(gm) = m.groups.get(&g) {
            let len = view_json(sim, "members_len", json!({ "g": g })).map_err(|e| Fail::new(format!("members_len: {e}")))?;
            if len.as_u64() != Some(gm.members.len() as u64) {
                return Err(Fail::new(format!("members_len({g}) = {len}, model {}", gm.members.len())).key("members_len"));
            }
        }
    }
    Ok(())
}

/// The developer's intent for the re-used tag: `claimable` is what schema B wrote, else 0.
fn check_retag(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    let d = deployed(sim).ok_or("no version")?;
    if !d.schema_b() || !m.migrated {
        return Ok(());
    }
    for (g, gm) in &m.groups {
        let Ok(got) = view_json(sim, "group", json!({ "g": g })) else { continue };
        let intent = if gm.prov4 == Prov::Claimable { u64_of(&gm.tags, 4).unwrap_or(0) } else { 0 };
        let claimable = got["claimable"].as_u64().unwrap_or(0);
        if claimable != intent {
            let msg = format!("{g}.claimable = {claimable} through {d:?}, but schema B never wrote it (v2a's `locked` at tag 4 was {:?})", u64_of(&gm.tags, 4));
            let cause = if gm.prov4 == Prov::Locked { "tag 4 re-used: v2a's locked bytes read as claimable" } else { "other" };
            return Err(Fail::new(msg).cause(cause));
        }
    }
    Ok(())
}

fn check_storage_usage(sim: &Sim) -> Result<(), Fail> {
    let d = deployed(sim).ok_or("no version")?;
    let usage = sim.account(APP).unwrap().storage_usage;
    let want = BASE_BYTES.get().unwrap() + d.wasm().len() as u64 + state_bytes(sim);
    if usage != want {
        return Err(Fail::new(format!("storage_usage {usage}, account + code + Σ(key + value + 40) = {want}")));
    }
    Ok(())
}

/// The new side's subtree, as the spec's staked-bytes table says a fresh `Map` holds it.
fn check_item_bytes(sim: &Sim) -> Result<(), Fail> {
    let (m, _) = fold(sim);
    if !m.migrated {
        return Ok(());
    }
    let mut want = 0u64;
    for (k, (_, side)) in &m.items {
        if *side == Side::New {
            let key = (3 + 16 + 4 + k.len()) as u64;
            want += key + 8 + 40; // entry
            want += key + 4 + 40; // index
            want += 7 + 4 + k.len() as u64 + 40; // key slot
        }
    }
    if m.new_len > 0 || m.new_rev {
        want += 3 + if m.new_rev { 12 } else { 4 } + 40;
    }
    let got: u64 = sim.view_state(APP, &[ROOT, 1]).iter().map(|(k, v)| (k.len() + v.len() + 40) as u64).sum();
    if got != want {
        return Err(Fail::new(format!("items subtree {got} bytes, spec formula {want} ({} on the new side, revision bumped {})", m.new_len, m.new_rev)));
    }
    Ok(())
}

const EXPECTED_PANICS: &[&str] = &[
    "group exists",
    "no such group",
    "no v1 STATE",
    "is being cleared by `remove_step`",
    "the budget counts host removes and must be at least",
    "refusing to remove the record",
    "state: cannot decode root",
    "state: cannot decode value",
    "The contract has already been initialized",
    "n must be > 0",
];

/// Every failure of the op is one the contract documents, or an explored-gas or code mismatch.
fn failures_expected(d: &OpDone<UpOp>) -> Result<(), Fail> {
    for o in d.outcomes() {
        for r in &o.receipts {
            let Some(f) = r.failure() else { continue };
            match &f {
                Failure::OutOfGas | Failure::GasLimitExceeded => {}
                // A call that lands on code without the method (overlap with a deploy).
                Failure::Action(k) if format!("{k:?}").contains("MethodResolveError") => {}
                Failure::Panic(msg) if EXPECTED_PANICS.iter().any(|p| msg.contains(p)) => {}
                other => {
                    let text = other.to_string();
                    let key: String = text.chars().take(48).collect();
                    return Err(Fail::new(format!("{} failed: {text}", r.method_names.join(","))).key(key));
                }
            }
        }
    }
    Ok(())
}


// ---------------------------------------------------------------------------------------------
// coverage probe: what the cases reached that a single-call mock cannot (counted per op run,
// shrinking and replays included, so the counts are an upper bound on distinct cases)
// ---------------------------------------------------------------------------------------------

static REACHED: std::sync::Mutex<BTreeMap<String, u64>> = std::sync::Mutex::new(BTreeMap::new());

fn hit(what: &str) {
    *REACHED.lock().unwrap().entry(what.to_string()).or_default() += 1;
}

fn reached_summary() -> String {
    let r = REACHED.lock().unwrap();
    r.iter().map(|(k, v)| format!("  {k}: {v}\n")).collect()
}

fn ev_of(r: &near_contract_sim::ReceiptOutcome) -> Option<(u64, Ver)> {
    let ev = r.logs.iter().find_map(|l| l.strip_prefix("ev "))?;
    let mut it = ev.split(' ');
    let v = Ver::parse(it.next()?.strip_prefix("v=")?)?;
    let seq = it.next()?.strip_prefix("seq=")?.parse().ok()?;
    Some((seq, v))
}

/// Never fails: counts the interactions each op's receipts reached.
fn probe(d: &OpDone<UpOp>) -> Result<(), Fail> {
    let evs = events(d.world());
    for o in d.outcomes() {
        let mut first_ver = None;
        for (i, r) in o.receipts.iter().filter(|r| r.executor_id.as_str() == APP && !r.method_names.is_empty()).enumerate() {
            if r.fault == Some(near_contract_sim::Fault::OutOfGas) {
                hit(&format!("starved {}", r.method_names.join(",")));
            }
            match r.failure() {
                Some(Failure::Panic(m)) => {
                    for (p, what) in [
                        ("is being cleared", "refused: write below a clearing record"),
                        ("refusing to remove", "refused: A3 unknown collection tag on removal"),
                        ("cannot decode value", "refused: record decode (v2c required tag)"),
                        ("cannot decode root", "refused: v2 code before migrate"),
                        ("must be at least", "refused: remove_step below 3"),
                        ("already been initialized", "refused: migrate twice"),
                        ("no v1 STATE", "refused: migrate with no STATE"),
                    ] {
                        if m.contains(p) {
                            hit(what);
                        }
                    }
                }
                Some(Failure::OutOfGas) => hit(&format!("ran out of explored gas: {}", r.method_names.join(","))),
                Some(Failure::Action(_)) => hit("method missing on the deployed code"),
                _ => {}
            }
            if !r.is_success() {
                continue;
            }
            let Some((seq, v)) = ev_of(r) else { continue };
            if i == 0 {
                first_ver = Some(v);
            } else if first_ver.is_some_and(|f| f != v) {
                hit(&format!("continuation ran on other code: {} {:?} -> {:?}", r.method_names.join(","), first_ver.unwrap(), v));
            } else if i > 0 {
                hit(&format!("continuation ran: {}", r.method_names.join(",")));
            }
            // The model just before this event.
            let mut m = Model::default();
            for e in evs.iter().take_while(|e| e.seq < seq) {
                m.apply(e);
            }
            let Some(e) = evs.iter().find(|e| e.seq == seq) else { continue };
            let g = s(&e.args, "g");
            if let Some(gm) = m.groups.get(&g) {
                let rewrite = matches!(e.method.as_str(), "set_weight" | "set_field4" | "set_level" | "set_label");
                if rewrite && v == Ver::V2a && (gm.tags.contains_key(&6) || gm.tags.contains_key(&7)) {
                    hit("v2a rewrote a record carrying schema-B tags (downgrade)");
                }
                if rewrite && v.schema_b() && gm.tags.contains_key(&3) {
                    hit("schema B rewrote a record carrying reserved tag 3");
                }
                if rewrite && v.schema_b() && gm.prov4 == Prov::Locked {
                    hit("schema B rewrote a record whose tag 4 is v2a's locked");
                }
                if matches!(e.method.as_str(), "remove_step" | "clear_loop") {
                    hit(if gm.clearing { "remove_step resumed a clearing record" } else { "remove_step started" });
                    if gm.clearing && v.schema_b() != m.groups[&g].tags.contains_key(&8) {
                        hit("remove_step resumed under another schema");
                    }
                }
                if gm.clearing && matches!(e.method.as_str(), "remove_group") {
                    hit("remove_group of a clearing record");
                }
            }
            if matches!(e.method.as_str(), "migrate_batch" | "drain") && m.v1_order.is_empty() {
                hit("migrate_batch on an empty old side");
            }
            if e.method == "insert" && v.migrating() && matches!(m.items.get(&s(&e.args, "k")), Some((_, Side::Old))) {
                hit("insert moved a key off the old side");
            }
        }
    }
    Ok(())
}

fn harness(world: Sim) -> Harness<UpOp> {
    Harness::new(world)
        .per_op("failures are documented ones", failures_expected)
        .per_op("coverage probe (never fails)", probe)
        .at_rest("seq has no gap", check_seq)
        .at_rest("the model saw nothing forbidden", check_violations)
        .at_rest("items read back through the deployed code", check_items_read_back)
        .at_rest("each item lives on exactly one side", check_one_side)
        .at_rest("group records match the model byte for byte", check_group_records)
        .at_rest("group views match the model", check_group_views)
        .at_rest("claimable is only what schema B wrote", check_retag)
        .at_rest("no collection outlives its record", check_no_orphans)
        .at_rest("storage_usage is account + code + every key", check_storage_usage)
        .at_rest("item subtree bytes match the spec formula", check_item_bytes)
}

fn retag_known() -> Known {
    Known::new("claimable is only what schema B wrote")
        .cause("tag 4 re-used: v2a's locked bytes read as claimable")
        .because("v2b re-uses tag 4 on purpose (spec: reusing a tag is a build error against the deployed ABI; --no-abi skips that check)")
        .leaves("v2a's `locked` value readable and writable as `claimable` in every pre-upgrade record")
}

fn a3_hole_known() -> Known {
    Known::new("no collection outlives its record")
        .cause("A3 hole: a collection written through at_mut under a record without its marker")
        .because("FOUND Oct 20 (a3_marker_missing_after_at_mut_write_orphans_a_new_collection): the kind marker is written only when the record is rewritten; at_mut writes below without rewriting, so v2a's A3 check cannot see v2b's flags")
        .leaves("the flag entries of a v2a-written record, orphaned on removal through v2a and inherited by the next record at that key")
}

/// SDK refusals a detached continuation can hit after a redeploy (it then fails with nobody
/// to tell): the A3 refusal, v2c's undecodable records, the clearing guard, code without the
/// method.
fn continuation_refusal(f: &near_contract_sim::explore::Seen) -> bool {
    f.outcome().is_some_and(|o| {
        o.receipts.iter().filter(|r| r.method_names.iter().any(|m| m == "clear_loop" || m == "drain")).any(|r| match r.failure() {
            Some(Failure::Panic(m)) => {
                ["refusing to remove", "Missing { tag: 9 }", "is being cleared", "cannot decode root"].iter().any(|p| m.contains(p))
            }
            Some(Failure::Action(k)) => format!("{k:?}").contains("MethodResolveError"),
            _ => false,
        })
    })
}

fn continuation_known(site: &str) -> Known {
    Known::new(near_contract_sim::builtin::UNHANDLED_FAILURE)
        .at(site)
        .when("a detached continuation landed on code that refuses (documented SDK refusal)", continuation_refusal)
        .because("drain/clear_loop chain themselves with detached self-calls; after a redeploy the next link can meet A3, v2c's required tag or a missing method, and the receipt fails atomically")
        .leaves("nothing: the failed link changes no state; the chain stops and the next call reports the refusal")
}

fn audited(world: Sim) -> Harness<UpOp> {
    harness(world)
        .known(retag_known())
        .known(a3_hole_known())
        .known(continuation_known("app.near::clear_loop"))
        .known(continuation_known("app.near::drain"))
        .out_of_scope("app.near::drain", "a public entry point the contract calls itself; Drain ops witness it")
        .out_of_scope("app.near::clear_loop", "a public entry point the contract calls itself; ClearLoop ops witness it")
}

// ---------------------------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------------------------

fn up(to: Ver, migrate: bool) -> UpOp {
    UpOp::Upgrade { to, migrate }
}

/// The fixture judges clean, and a plain upgrade path does.
#[test]
fn smoke_upgrade_path() {
    let plan = Plan::new()
        .op(UpOp::Insert { k: 0, v: 5 })
        .op(up(Ver::V2a, true))
        .op(UpOp::Insert { k: 1, v: 6 })
        .op(UpOp::Insert { k: 6, v: 9 })
        .op(UpOp::CreateGroup { g: 0, weight: 3, x: 0 })
        .op(UpOp::AddMember { g: 0, m: 1, v: 2 })
        .op(UpOp::MigrateBatch { n: 5 })
        .op(UpOp::Drain { n: 4, rounds: 5 })
        .op(up(Ver::V2b, false))
        .op(UpOp::SetLevel { g: 0, x: 3 })
        .op(UpOp::AddFlag { g: 0, f: 1 })
        .op(up(Ver::V3, false))
        .op(UpOp::Remove { k: 1 })
        .op(UpOp::ClearLoop { g: 0, n: 3, rounds: 6 });
    let t = std::time::Instant::now();
    let run = harness(v1_world()).run(&plan);
    println!("{run}\n{:?}", t.elapsed());
    run.assert_clean();
}

/// Positive control: the re-used tag is caught, with its cause, and only that.
#[test]
fn retag_hazard_is_caught() {
    let plan = Plan::new()
        .op(up(Ver::V2a, true))
        .op(UpOp::CreateGroup { g: 0, weight: 3, x: 9 })
        .op(up(Ver::V2b, false));
    let run = harness(v1_world()).run(&plan);
    println!("{run}");
    let f = run.assert_found("claimable is only what schema B wrote", "world");
    println!("{}", f.message);
    let intended = plan.clone();
    // A different failure of the same check: none exists in a correct build, so the second
    // control uses a plan whose claimable was written by v2b and must stay clean.
    let different = Plan::new()
        .op(up(Ver::V2b, true))
        .op(UpOp::CreateGroup { g: 0, weight: 3, x: 9 })
        .op(UpOp::SetField4 { g: 0, x: 4 });
    harness(v1_world()).run(&different).assert_clean();
    harness(v1_world()).known(retag_known()).run(&intended);
}

/// Positive controls for the model's raw checks: corrupt state and watch them fail.
#[test]
fn raw_checks_can_fail() {
    let mut sim = v2a_world();
    assert!(check_group_records(&sim).is_ok());
    assert!(check_one_side(&sim).is_ok());
    let mut k = group_rec("g0");
    k.extend([5, 0]);
    k.extend(entry_seg("m4"));
    sim.set_state(APP, &k, Some(&7u64.to_le_bytes()));
    assert!(check_group_records(&sim).is_err());
    let mut sim = v2a_world();
    sim.set_state(APP, &item_v2("s00"), Some(&1000u64.to_le_bytes()));
    assert!(check_one_side(&sim).is_err());
}

/// "storage_usage after full migration equals a fresh v2 contract with the same content."
#[test]
fn drained_contract_matches_a_fresh_v2_one() {
    let mut a = v1_world();
    a.tx(APP, APP).deploy(Ver::V2a.wasm()).call("migrate").transact().assert_success();
    for _ in 0..4 {
        a.call(APP, APP, "migrate_batch").args_json(json!({ "n": 5 })).transact().assert_success();
    }
    assert_eq!(a.view(APP, "old_is_empty", json!({})).json::<bool>(), true);
    a.tx(APP, APP).deploy(Ver::V3.wasm()).transact().assert_success();

    let mut b = Sim::new();
    b.create_account(APP, Balance::from_near(100));
    b.deploy(APP, Ver::V3.wasm().to_vec());
    for i in 0..SEED {
        b.call(APP, APP, "insert").args_json(json!({ "k": format!("s{i:02}"), "v": 1000 + i })).transact().assert_success();
    }
    let sa = a.view_state(APP, &[]);
    let sb = b.view_state(APP, &[]);
    let items = |s: &[(Vec<u8>, Vec<u8>)]| s.iter().filter(|(k, _)| k.starts_with(&[ROOT, 1])).cloned().collect::<BTreeMap<_, _>>();
    let (ia, ib) = (items(&sa), items(&sb));
    // Slot order differs (batch order vs insertion order), bytes do not.
    let bytes = |m: &BTreeMap<Vec<u8>, Vec<u8>>| m.iter().map(|(k, v)| k.len() + v.len() + 40).sum::<usize>();
    println!("items subtree: drained {} bytes in {} keys, fresh {} bytes in {} keys", bytes(&ia), ia.len(), bytes(&ib), ib.len());
    // Entries and header are byte-identical; index values and key slots hold slot positions.
    let entries = |m: &BTreeMap<Vec<u8>, Vec<u8>>| m.iter().filter(|(k, _)| k[2] == 0 || k[2] == 3).map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>();
    assert_eq!(entries(&ia), entries(&ib), "entries and header identical");
    let keys = |m: &BTreeMap<Vec<u8>, Vec<u8>>| m.keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(&ia), keys(&ib), "the same keys");
    assert_eq!(bytes(&ia), bytes(&ib));
    let root_a = &sa.iter().find(|(k, _)| k == &vec![ROOT]).unwrap().1;
    let root_b = &sb.iter().find(|(k, _)| k == &vec![ROOT]).unwrap().1;
    let (ua, ub) = (a.account(APP).unwrap().storage_usage, b.account(APP).unwrap().storage_usage);
    println!("root record: drained {} bytes {:02x?}, fresh {} bytes {:02x?}", root_a.len(), root_a, root_b.len(), root_b);
    println!("storage_usage: drained {ua}, fresh {ub}, delta {}", ua as i64 - ub as i64);
    let other = |s: &[(Vec<u8>, Vec<u8>)]| s.iter().filter(|(k, _)| !k.starts_with(&[ROOT, 1]) && k != &vec![ROOT]).count();
    assert_eq!((other(&sa), other(&sb)), (0, 0), "nothing else stored");
    assert_eq!(ua - ub, (root_a.len() - root_b.len()) as u64, "the whole difference is the root record");
}

fn search_report(world: Sim, upgrade_weight: u32, cases: u32, len: std::ops::RangeInclusive<usize>) -> near_contract_sim::explore::Report<UpOp> {
    let t = std::time::Instant::now();
    let report = audited(world).search(plans(ops(upgrade_weight)).len(len)).cases(cases).run();
    let el = t.elapsed();
    println!("{report}");
    println!(
        "WALL {:.1} s for {} cases: {:.0} ms per case",
        el.as_secs_f64(),
        report.coverage().cases(),
        el.as_millis() as f64 / report.coverage().cases().max(1) as f64
    );
    println!("reached (op runs, shrinking included):\n{}", reached_summary());
    report
}

#[test]
#[ignore = "long: NEAR_SIM_CASES and NEAR_SIM_SEED tune it"]
fn search_from_v1() {
    let report = search_report(v1_world(), 10, 256, 4..=14);
    report.assert_no_new_findings();
}

#[test]
#[ignore = "long: NEAR_SIM_CASES and NEAR_SIM_SEED tune it"]
fn search_from_v2a() {
    let report = search_report(v2a_world(), 3, 256, 4..=14);
    report.assert_no_new_findings();
}

#[test]
#[ignore = "long: NEAR_SIM_CASES and NEAR_SIM_SEED tune it"]
fn search_from_v2b() {
    let report = search_report(v2b_world(), 4, 256, 4..=14);
    report.assert_no_new_findings();
}

/// Every fault at every receipt of witness plans that run the self-call continuations, with a
/// deploy landing between continuation receipts.
#[test]
#[ignore = "long"]
fn scan_out_of_gas() {
    let witnesses = [
        Plan::new().op(UpOp::Drain { n: 3, rounds: 5 }).op(UpOp::Insert { k: 6, v: 1 }),
        Plan::new()
            .op(UpOp::Drain { n: 2, rounds: 6 })
            .blocks(1)
            .op(up(Ver::V2b, false))
            .op(UpOp::MigrateBatch { n: 20 }),
        Plan::new()
            .op(UpOp::AddMember { g: 1, m: 4, v: 1 })
            .op(UpOp::AddMember { g: 1, m: 5, v: 1 })
            .op(UpOp::ClearLoop { g: 0, n: 3, rounds: 8 })
            .op(UpOp::ClearLoop { g: 1, n: 4, rounds: 8 }),
        Plan::new()
            .op(UpOp::ClearLoop { g: 0, n: 3, rounds: 8 })
            .blocks(1)
            .op(up(Ver::V2b, false))
            .op(UpOp::AddMember { g: 0, m: 0, v: 3 })
            .op(UpOp::RemoveStep { g: 0, n: 12 }),
        Plan::new()
            .op(up(Ver::V2b, false))
            .op(UpOp::AddFlag { g: 0, f: 0 })
            .op(UpOp::ClearLoop { g: 0, n: 3, rounds: 4 })
            .blocks(1)
            .op(up(Ver::V2a, false))
            .op(UpOp::RemoveStep { g: 0, n: 12 }),
        // The deploy lands in the block the first link runs in, so every later link of the
        // chain runs on the new code (cross-version continuation).
        Plan::new()
            .op(UpOp::ClearLoop { g: 0, n: 3, rounds: 8 })
            .blocks(0)
            .op(up(Ver::V2b, false))
            .op(UpOp::AddMember { g: 1, m: 3, v: 1 }),
        Plan::new()
            .op(UpOp::Drain { n: 2, rounds: 8 })
            .blocks(0)
            .op(up(Ver::V2b, false))
            .op(UpOp::Insert { k: 7, v: 1 }),
        Plan::new()
            .op(up(Ver::V2b, false))
            .op(UpOp::CreateGroup { g: 2, weight: 1, x: 1 })
            .op(UpOp::AddFlag { g: 2, f: 1 })
            .op(UpOp::ClearLoop { g: 2, n: 3, rounds: 6 })
            .blocks(0)
            .op(up(Ver::V2a, false)),
    ];
    let t = std::time::Instant::now();
    let report = audited(v2a_world()).scan_each(&witnesses, Faults::default());
    let el = t.elapsed();
    println!("{report}");
    println!("WALL {:.1} s for the scan", el.as_secs_f64());
    println!("reached (op runs):\n{}", reached_summary());
    report.assert_no_new_findings();
}

fn tx_ok(sim: &mut Sim, method: &str, args: Value) -> Result<Vec<u8>, String> {
    let o = sim.call(APP, APP, method).args_json(args).transact();
    match o.failure() {
        None => Ok(o.value().unwrap_or_default().to_vec()),
        Some(f) => Err(f.to_string()),
    }
}

/// v2b -> v2a -> v2b: v2a rewrites the record (get_mut) and keeps the tags it does not know.
#[test]
fn downgrade_keeps_schema_b_fields() {
    let mut sim = v2b_world();
    let before: Value = sim.view(APP, "group", json!({ "g": "g0" })).json();
    sim.tx(APP, APP).deploy(Ver::V2a.wasm()).transact().assert_success();
    let in_a: Value = sim.view(APP, "group", json!({ "g": "g0" })).json();
    tx_ok(&mut sim, "set_weight", json!({ "g": "g0", "w": 99 })).unwrap();
    tx_ok(&mut sim, "add_member", json!({ "g": "g0", "m": "m5", "v": 1 })).unwrap();
    sim.tx(APP, APP).deploy(Ver::V2b.wasm()).transact().assert_success();
    let after: Value = sim.view(APP, "group", json!({ "g": "g0" })).json();
    println!("v2b before: {before}\nv2a sees:   {in_a}\nv2b after:  {after}");
    assert_eq!(after["level"], before["level"]);
    assert_eq!(after["label"], before["label"]);
    assert_eq!(after["weight"], 99);
    assert_eq!(in_a["note"], "", "tag 3 was dropped by v2b (reserved), so v2a reads its default");
    let run = audited(sim).run(&Plan::new().op(UpOp::Insert { k: 0, v: 1 }));
    assert!(run.findings().is_empty(), "{run}");
}

/// What each method does on a record written before v2c's `required` tag existed.
#[test]
fn required_tag_on_old_records() {
    let mut sim = v2a_world();
    sim.tx(APP, APP).deploy(Ver::V2c.wasm()).transact().assert_success();
    let mut lines = vec![];
    let view = sim.view(APP, "group", json!({ "g": "g0" }));
    lines.push(format!("view group(g0): {:?}", view.result.map(|b| String::from_utf8_lossy(&b).to_string())));
    let len = sim.view(APP, "members_len", json!({ "g": "g0" }));
    lines.push(format!("view members_len(g0): {:?}", len.result.map(|b| String::from_utf8_lossy(&b).to_string())));
    for (m, a) in [
        ("set_weight", json!({ "g": "g0", "w": 1 })),
        ("add_member", json!({ "g": "g0", "m": "m4", "v": 1 })),
        ("add_flag", json!({ "g": "g0", "f": "f1" })),
        ("remove_member", json!({ "g": "g0", "m": "m0" })),
        ("remove_step", json!({ "g": "g0", "n": 3 })),
        ("remove_group", json!({ "g": "g0" })),
        ("create_group", json!({ "g": "g2", "weight": 1, "x": 1 })),
    ] {
        let r = tx_ok(&mut sim, m, a);
        lines.push(format!("{m}: {:?}", r.map(|b| String::from_utf8_lossy(&b).to_string()).map_err(|e| e.chars().take(110).collect::<String>())));
    }
    for l in &lines {
        println!("{l}");
    }
    assert!(lines[0].contains("Err"), "an old record does not decode under v2c");
    assert!(lines.iter().any(|l| l.starts_with("add_member: Ok")), "at_mut does not decode, so writes below still land");
    assert!(lines.iter().any(|l| l.starts_with("remove_group: Err")), "removal decodes, so the record cannot be removed");
    let run = audited(sim).run(&Plan::new().op(UpOp::Insert { k: 0, v: 1 }));
    assert!(run.findings().is_empty(), "{run}");
}

/// The re-used tag, both directions: v2a's `locked` read as v2b's `claimable`, and v2b's
/// `claimable` read back as v2a's `locked` after a rollback.
#[test]
fn retag_both_directions() {
    let mut sim = v2a_world(); // g0: locked = 5
    sim.tx(APP, APP).deploy(Ver::V2b.wasm()).transact().assert_success();
    let b: Value = sim.view(APP, "group", json!({ "g": "g0" })).json();
    tx_ok(&mut sim, "set_field4", json!({ "g": "g1", "x": 42 })).unwrap(); // g1.claimable = 42
    sim.tx(APP, APP).deploy(Ver::V2a.wasm()).transact().assert_success();
    let a: Value = sim.view(APP, "group", json!({ "g": "g1" })).json();
    println!("v2b reads g0 (created by v2a, locked 5): {b}\nv2a reads g1 after v2b set claimable 42: {a}");
    assert_eq!(b["claimable"], 5);
    assert_eq!(a["locked"], 42);
}

/// FOUND by `scan_out_of_gas` (witness 5), minimised by hand: a record written by v2a carries
/// no marker for v2b's `flags` (tag 8). v2b adds a flag through `at_mut`, which writes below the
/// record without rewriting it, so the marker is never written. Rolled back to v2a, removing
/// the record passes the A3 check (no unknown marker in its bytes) and leaves the flag entries
/// behind; a new g0 then inherits them once v2b is back: exactly hazard A3.
#[test]
fn a3_marker_missing_after_at_mut_write_orphans_a_new_collection() {
    let mut sim = v2a_world(); // g0 written by v2a: tags 1-5, no tag 8
    sim.tx(APP, APP).deploy(Ver::V2b.wasm()).transact().assert_success();
    tx_ok(&mut sim, "add_flag", json!({ "g": "g0", "f": "f0" })).unwrap();
    let rec = raw(&sim, &group_rec("g0")).unwrap();
    println!("g0 record after v2b add_flag: {rec:02x?} (no 0x21 = tag 8 marker field key)");
    sim.tx(APP, APP).deploy(Ver::V2a.wasm()).transact().assert_success();
    let removed = tx_ok(&mut sim, "remove_group", json!({ "g": "g0" }));
    println!("v2a remove_group(g0): {removed:?}");
    let left = sim.view_state(APP, &group_rec("g0"));
    println!("keys left under g0: {}", left.len());
    tx_ok(&mut sim, "create_group", json!({ "g": "g0", "weight": 1, "x": 1 })).unwrap();
    sim.tx(APP, APP).deploy(Ver::V2b.wasm()).transact().assert_success();
    let inherited = sim.state(APP, &coll_entry("g0", 8, "f0")).is_some();
    println!("new g0 under v2b sees the old flag f0: {inherited}");
    if removed.is_ok() {
        assert!(!left.is_empty() && inherited, "the hole: removal passed and orphaned the flags");
        println!("HOLE PRESENT (production code without the fix)");
    } else {
        assert!(removed.unwrap_err().contains("refusing to remove"), "fixed: removal refuses");
        println!("FIXED: v2a refuses the removal");
    }
}

/// The scan's saved case (`tests/found/a3-orphaned-flags.json`): on this build the A3 hole
/// shows; once fixed, the removal through v2a is refused and the case is clean of it.
#[test]
fn a3_orphan_replay() {
    let path = format!("{}/tests/found/a3-orphaned-flags.json", env!("CARGO_MANIFEST_DIR"));
    let run = harness(v2a_world()).rerun(&path);
    println!("{run}");
    run.assert_found("no collection outlives its record", "world");
}
