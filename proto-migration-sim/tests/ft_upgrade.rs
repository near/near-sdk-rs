//! Storage v2 "move the root", for real: nearcore's runtime via near-contract-sim.
//!
//! Wasms in tests/res, built from the worktree with `cargo near build non-reproducible-wasm
//! --no-abi` (see PROTOTYPE_NOTES.md, "Migration round"):
//! - ft_v1.wasm: examples/fungible-token/ft, unmodified;
//! - ft_v2.wasm: examples/fungible-token/ft-v2 (v2 root, v1 FungibleToken kept at the root);
//! - ft_v3.wasm: the same crate with `--features v3` (accounts in `Migrating`).

use near_contract_sim::prelude::*;
use serde_json::json;

const FT: &str = "ft.near";
const USERS: usize = 300;
const SUPPLY: u128 = 1_000_000_000_000;
const ROOT: u8 = 0xF0;
const ICON_BYTES: usize = 100 * 1024;

fn res(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/res/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}
fn user(i: usize) -> String {
    format!("user{i:03}.near")
}
fn expected(i: usize) -> u128 {
    1_000 + i as u128
}

/// v1 deployed and populated: 100 KiB icon, 300 registered and funded accounts.
fn v1_world() -> Sim {
    let mut sim = Sim::new();
    sim.create_account(user(1).as_str(), Balance::from_near(10));
    sim.create_account(user(2).as_str(), Balance::from_near(10));
    sim.deploy(FT, res("ft_v1.wasm"));
    let icon = format!("data:image/svg+xml,{}", "x".repeat(ICON_BYTES));
    sim.call(FT, FT, "new")
        .args_json(json!({
            "owner_id": FT,
            "total_supply": SUPPLY.to_string(),
            "metadata": {"spec": "ft-1.0.0", "name": "Example", "symbol": "EX", "icon": icon, "decimals": 24}
        }))
        .transact()
        .assert_success();
    let bound: serde_json::Value = sim.view(FT, "storage_balance_bounds", json!({})).json();
    let bond = Balance::from_yoctonear(bound["min"].as_str().unwrap().parse().unwrap());
    for i in 0..USERS {
        sim.call(FT, FT, "storage_deposit").args_json(json!({"account_id": user(i)})).deposit(bond).submit();
    }
    sim.settle();
    for i in 0..USERS {
        sim.call(FT, FT, "ft_transfer")
            .args_json(json!({"receiver_id": user(i), "amount": expected(i).to_string()}))
            .deposit(Balance::from_yoctonear(1))
            .submit();
    }
    sim.settle();
    sim
}

fn balance(sim: &Sim, who: &str) -> u128 {
    let v: String = sim.view(FT, "ft_balance_of", json!({"account_id": who})).json();
    v.parse().unwrap()
}

fn transfer(sim: &mut Sim, from: &str, to: &str, amount: u128) -> TxOutcome {
    let out = sim
        .call(from, FT, "ft_transfer")
        .args_json(json!({"receiver_id": to, "amount": amount.to_string()}))
        .deposit(Balance::from_yoctonear(1))
        .transact();
    out.assert_success();
    out
}

fn report(label: &str, out: &TxOutcome) -> (u64, u64) {
    let r = out.first_call().unwrap();
    let s = r.storage_ops();
    println!(
        "{label}: {:.3} Tgas burnt by the call ({:.3} total); storage r/w/d/has {}/{}/{}/{}, read {} B, written {} B, trie nodes {}",
        r.gas_burnt.as_gas() as f64 / 1e12,
        out.gas_burnt().as_gas() as f64 / 1e12,
        s.reads,
        s.writes,
        s.removes,
        s.has_keys,
        s.read_bytes,
        s.written_bytes,
        s.trie_nodes
    );
    for w in &s.written {
        println!("    wrote {w}");
    }
    let mut prof: Vec<_> = r.gas_profile.iter().filter(|(_, g)| **g > 20_000_000_000).collect();
    prof.sort_by(|a, b| b.1.cmp(a.1));
    println!("    profile >20 Ggas: {}", prof.iter().map(|(k, g)| format!("{k} {:.0}G", **g as f64 / 1e9)).collect::<Vec<_>>().join(", "));
    (r.gas_burnt.as_gas(), s.read_bytes)
}

#[test]
fn move_the_root_on_the_runtime() {
    let mut sim = v1_world();
    let before = sim.view_state(FT, &[]);
    println!("v1 state: {} keys", before.len());
    assert_eq!(before.len(), USERS + 1 + 2);
    assert!(before.iter().all(|(k, _)| k[0] != ROOT), "a v1 key starts with the root byte");
    assert!(before.iter().any(|(k, v)| k == &[1u8] && v.len() > ICON_BYTES));

    // v1 baseline: transfer between two old accounts.
    let mut v1_world2 = sim.clone();
    let (v1_gas, _) = report("v1 ft_transfer", &transfer(&mut v1_world2, &user(1), &user(2), 10));

    // Upgrade: deploy v2 and call migrate in ONE transaction (no init window).
    let out = sim.tx(FT, FT).deploy(res("ft_v2.wasm")).call("migrate").transact();
    out.assert_success();
    report("deploy v2 + migrate", &out);

    let after = sim.view_state(FT, &[]);
    assert!(!after.iter().any(|(k, _)| k == b"STATE"), "STATE removed");
    assert!(after.iter().any(|(k, _)| k == &[ROOT]), "root record written");
    let old: std::collections::BTreeMap<_, _> = before.into_iter().filter(|(k, _)| k != b"STATE").collect();
    for (k, v) in &old {
        assert_eq!(after.iter().find(|(k2, _)| k2 == k).map(|(_, v2)| v2), Some(v), "v1 key {k:02x?} changed");
    }
    assert_eq!(after.len(), old.len() + 1);

    for i in 0..USERS {
        assert_eq!(balance(&sim, &user(i)), expected(i), "{}", user(i));
    }
    let supply: String = sim.view(FT, "ft_total_supply", json!({})).json();
    assert_eq!(supply, SUPPLY.to_string());

    let (v2_first, rb) = report("v2 ft_transfer (first by sender: new v2 Map key)", &transfer(&mut sim, &user(1), &user(2), 10));
    assert!(rb < 2_000, "the 100 KiB metadata is not read");
    let (v2_second, _) = report("v2 ft_transfer (second)", &transfer(&mut sim, &user(1), &user(2), 10));
    assert_eq!(balance(&sim, &user(1)), expected(1) - 20);
    assert_eq!(balance(&sim, &user(2)), expected(2) + 20);
    let n: u64 = sim.view(FT, "transfers_sent_by", json!({"account_id": user(1)})).json();
    assert_eq!(n, 2);
    println!(
        "gas: v1 {:.3} Tgas, v2 first {:.3}, v2 second {:.3} (call receipt)",
        v1_gas as f64 / 1e12,
        v2_first as f64 / 1e12,
        v2_second as f64 / 1e12
    );
    let firsts: std::collections::BTreeSet<u8> = sim.view_state(FT, &[]).iter().map(|(k, _)| k[0]).collect();
    assert_eq!(firsts, [0x00, 0x01, ROOT].into());

    // Rekey: deploy v3 + migrate (from the v2 root), then transfers through `Migrating`.
    let out = sim.tx(FT, FT).deploy(res("ft_v3.wasm")).call("migrate").transact();
    out.assert_success();
    report("deploy v3 + migrate", &out);
    assert_eq!(balance(&sim, &user(5)), expected(5));
    let (m1, _) = report("v3 ft_transfer, both old-side (moves 2 keys)", &transfer(&mut sim, &user(1), &user(2), 1));
    let (m2, _) = report("v3 ft_transfer, both migrated", &transfer(&mut sim, &user(1), &user(2), 1));
    assert_eq!(balance(&sim, &user(1)), expected(1) - 22);
    assert!(!sim.view_state(FT, &[0u8]).iter().any(|(k, _)| k[1..] == borsh_account(&user(1))[..]));
    println!("gas: v3 moving {:.3} Tgas, v3 migrated {:.3}", m1 as f64 / 1e12, m2 as f64 / 1e12);
}

fn borsh_account(a: &str) -> Vec<u8> {
    let mut v = (a.len() as u32).to_le_bytes().to_vec();
    v.extend(a.as_bytes());
    v
}

/// Deploy v2 alone, without calling migrate: the contract is broken until migrate runs
/// (views fail decoding the empty root), and the init guard sees no v2 root.
#[test]
fn deploy_without_migrate_window() {
    let mut sim = v1_world();
    sim.deploy(FT, res("ft_v2.wasm"));
    let v = sim.view(FT, "ft_total_supply", json!({}));
    println!("view before migrate: {:?}", v.result.as_ref().err());
    assert!(v.result.is_err());
    // migrate is #[private]: a stranger cannot call it.
    let out = sim.call(&user(1), FT, "migrate").transact();
    assert!(out.is_failure());
    println!("stranger migrate: {:?}", out.failure());
    sim.call(FT, FT, "migrate").transact().assert_success();
    assert_eq!(balance(&sim, &user(7)), expected(7));
    // and only once
    let again = sim.call(FT, FT, "migrate").transact();
    println!("second migrate: {:?}", again.failure());
    assert!(again.is_failure());
}
