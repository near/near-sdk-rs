// As wasm VM performance is tested, there is no need to test this on other types of OS.
// This test runs only on Linux, as it's much slower on OS X due to an interpreted VM.
#![cfg(target_os = "linux")]

use near_account_id::AccountId;
use near_gas::NearGas;
use near_workspaces::network::Sandbox;
use near_workspaces::types::{KeyType, SecretKey};
use near_workspaces::{Account, Worker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum_macros::Display;

const DEFAULT_INDEX_OFFSET: usize = 0;

#[derive(Serialize, Deserialize, Display, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(crate = "near_sdk::serde")]
pub enum Collection {
    IterableSet,
    IterableMap,
    UnorderedSet,
    UnorderedMap,
    LookupMap,
    LookupSet,
    TreeMap,
    Vector,
}

fn random_account_id(collection: Collection, seed: &str) -> AccountId {
    let mut rng = rand::thread_rng();
    let random_num = rng.gen_range(10000000000000usize..99999999999999);
    let account_id = format!(
        "dev-{}-{}-{}-{}",
        random_num,
        seed,
        random_num,
        collection.to_string().to_lowercase()
    );
    let account_id: AccountId =
        account_id.try_into().expect("could not convert dev account into AccountId");

    account_id
}
async fn dev_generate(
    worker: Arc<Worker<Sandbox>>,
    collection: Collection,
    seed: String,
) -> anyhow::Result<(Account, Collection)> {
    let id = random_account_id(collection, &seed);
    let sk = SecretKey::from_seed(KeyType::ED25519, &seed);
    let account = worker.create_tla(id.clone(), sk).await?;
    Ok((account.into_result()?, collection))
}

async fn setup_worker() -> anyhow::Result<(Arc<Worker<Sandbox>>, AccountId)> {
    let worker = Arc::new(near_workspaces::sandbox().await?);
    let wasm = near_workspaces::compile_project("./tests/test-contracts/store").await?;
    let contract = worker.dev_deploy(&wasm).await?;
    let res = contract.call("new").max_gas().transact().await?;
    assert!(res.is_success());
    Ok((worker, contract.id().clone()))
}

fn perform_asserts(total_gas: u64, col: &Collection) {
    // Constraints a bit relaxed to account for binary differences due to on-demand compilation.
    assert!(
        total_gas < NearGas::from_tgas(110).as_gas(),
        "performance regression {}: {}",
        col,
        NearGas::from_gas(total_gas)
    );
    assert!(
        total_gas > NearGas::from_tgas(90).as_gas(),
        "not enough gas consumed {}: {}, adjust the number of iterations to spot regressions",
        col,
        NearGas::from_gas(total_gas)
    );
}

#[allow(unused)]
async fn setup_several(num: usize) -> anyhow::Result<(Vec<Account>, AccountId)> {
    let (worker, contract_id) = setup_worker().await?;
    let mut accounts = Vec::new();

    for acc_seed in 0..num {
        let (account, _) =
            dev_generate(worker.clone(), Collection::IterableSet, acc_seed.to_string()).await?;
        accounts.push(account);
    }

    Ok((accounts, contract_id))
}

async fn setup() -> anyhow::Result<(Account, AccountId)> {
    let (worker, contract_id) = setup_worker().await?;

    let (account, _) =
        dev_generate(worker.clone(), Collection::IterableSet, "seed".to_string()).await?;

    Ok((account, contract_id))
}

#[tokio::test]
async fn insert_and_remove() -> anyhow::Result<()> {
    let collection_types = &[
        Collection::TreeMap,
        Collection::IterableSet,
        Collection::IterableMap,
        Collection::UnorderedSet,
        Collection::UnorderedMap,
        Collection::LookupMap,
        Collection::LookupSet,
        Collection::Vector,
    ];

    let (account, contract_id) = setup().await?;
    // insert test, max_iterations here is the number of elements to insert. It's used to measure
    // relative performance.
    for (col, max_iterations) in collection_types.map(|col| match col {
        Collection::TreeMap => (col, 365),
        Collection::IterableSet => (col, 370),
        Collection::IterableMap => (col, 370),
        Collection::UnorderedSet => (col, 360),
        Collection::UnorderedMap => (col, 365),
        Collection::LookupMap => (col, 650),
        Collection::LookupSet => (col, 1020),
        Collection::Vector => (col, 1080),
    }) {
        let total_gas = account
            .call(&contract_id, "insert")
            .args_json((col, DEFAULT_INDEX_OFFSET, max_iterations))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, &col);
    }

    // remove test, max_iterations here is the number of elements to remove. It's used to measure
    // relative performance.
    for (col, max_iterations) in collection_types.map(|col| match col {
        Collection::TreeMap => (col, 230),
        Collection::IterableSet => (col, 130),
        Collection::IterableMap => (col, 120),
        Collection::UnorderedSet => (col, 240),
        Collection::UnorderedMap => (col, 250),
        Collection::LookupMap => (col, 520),
        Collection::LookupSet => (col, 1050),
        Collection::Vector => (col, 530),
    }) {
        let total_gas = account
            .call(&contract_id, "remove")
            .args_json((col, max_iterations))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, &col);
    }

    Ok(())
}

#[tokio::test]
async fn iter() -> anyhow::Result<()> {
    // LookupMap and LookupSet are not iterable.
    let collection_types = &[
        Collection::TreeMap,
        Collection::IterableSet,
        Collection::IterableMap,
        Collection::UnorderedSet,
        Collection::UnorderedMap,
        Collection::Vector,
    ];

    let element_number = 100;
    let (account, contract_id) = setup().await?;

    // pre-populate
    for col in collection_types {
        account
            .call(&contract_id, "insert")
            .args_json((col, DEFAULT_INDEX_OFFSET, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap();
    }

    // iter, repeat here is the number that reflects how many times the iterator is consumed fully.
    // It's used to measure relative performance.
    for (col, repeat) in collection_types.map(|col| match col {
        Collection::TreeMap => (col, 5),
        Collection::IterableSet => (col, 22),
        Collection::IterableMap => (col, 10),
        Collection::UnorderedSet => (col, 20),
        Collection::UnorderedMap => (col, 9),
        Collection::Vector => (col, 22),
        _ => (col, 0),
    }) {
        let total_gas = account
            .call(&contract_id.clone(), "iter")
            .args_json((col, repeat, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, &col);
    }

    Ok(())
}

#[tokio::test]
async fn random_access() -> anyhow::Result<()> {
    // LookupMap and LookupSet are not iterable.
    let collection_types = &[
        Collection::TreeMap,
        Collection::IterableSet,
        Collection::IterableMap,
        Collection::UnorderedSet,
        Collection::UnorderedMap,
        Collection::Vector,
    ];
    let element_number = 100;
    let (account, contract_id) = setup().await?;

    // pre-populate
    for col in collection_types {
        account
            .call(&contract_id, "insert")
            .args_json((col, DEFAULT_INDEX_OFFSET, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap();
    }

    // Rust 1.81 improved performance of unordered collections.
    let unordered_map = if rustversion::cfg!(since(1.81)) { 42 } else { 36 };

    // iter, repeat here is the number that reflects how many times we retrieve a random element.
    // It's used to measure relative performance.
    for (col, repeat) in collection_types.map(|col| match col {
        Collection::TreeMap => (col, 15),
        Collection::IterableSet => (col, 1750),
        Collection::IterableMap => (col, 745),
        Collection::UnorderedSet => (col, 41),
        Collection::UnorderedMap => (col, unordered_map),
        Collection::Vector => (col, 1700),
        _ => (col, 0),
    }) {
        let total_gas = account
            .call(&contract_id.clone(), "nth")
            .args_json((col, repeat, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, &col);
    }

    Ok(())
}

#[tokio::test]
async fn contains() -> anyhow::Result<()> {
    // Vector does not implement contains.
    let collection_types = &[
        Collection::TreeMap,
        Collection::IterableSet,
        Collection::IterableMap,
        Collection::UnorderedSet,
        Collection::UnorderedMap,
        Collection::LookupMap,
        Collection::LookupSet,
    ];
    // Each collection gets the same number of elements.
    let element_number = 100;
    let (account, contract_id) = setup().await?;

    // prepopulate
    for col in collection_types {
        account
            .call(&contract_id, "insert")
            .args_json((col, DEFAULT_INDEX_OFFSET, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap();
    }

    // contains test, repeat here is the number of times we check all the elements in each collection.
    // It's used to measure relative performance.
    for (col, repeat) in collection_types.map(|col| match col {
        Collection::TreeMap => (col, 13),
        Collection::IterableSet => (col, 12),
        Collection::IterableMap => (col, 13),
        Collection::UnorderedSet => (col, 12),
        Collection::UnorderedMap => (col, 13),
        Collection::LookupMap => (col, 17),
        Collection::LookupSet => (col, 15),
        _ => (col, 0),
    }) {
        let total_gas = account
            .call(&contract_id.clone(), "contains")
            .args_json((col, repeat, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, &col);
    }

    Ok(())
}

// This test demonstrates the difference in gas consumption between iterable and unordered collections,
// when most of the elements have been deleted.
#[tokio::test]
async fn iterable_vs_unordered() -> anyhow::Result<()> {
    let element_number = 300;
    let deleted_element_number = 299;
    let (account, contract_id) = setup().await?;

    // We only care about Unordered* and Iterable* collections.
    let collection_types = &[
        Collection::UnorderedSet,
        Collection::UnorderedMap,
        Collection::IterableMap,
        Collection::IterableSet,
    ];

    // insert `element_number` elements.
    for col in collection_types {
        account
            .call(&contract_id, "insert")
            .args_json((col, DEFAULT_INDEX_OFFSET, element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap();
    }

    // remove `deleted_element_number` elements. This leaves only one element in each collection.
    for (col, max_iterations) in &collection_types.map(|col| (col, deleted_element_number)) {
        account
            .call(&contract_id, "remove")
            .args_json((col, max_iterations))
            .max_gas()
            .transact()
            .await?
            .unwrap();
    }

    // iter, repeat here is the number of times we iterate through the whole collection. It's used to
    // measure relative performance.
    for (col, repeat) in collection_types.map(|col| match col {
        Collection::IterableSet => (col, 260000),
        Collection::IterableMap => (col, 135000),
        Collection::UnorderedSet => (col, 280),
        Collection::UnorderedMap => (col, 270),
        _ => (col, 0),
    }) {
        let total_gas = account
            .call(&contract_id.clone(), "iter")
            .args_json((col, repeat, element_number - deleted_element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, &col);
    }

    // random access, repeat here is the number of times we try to access an element in the
    // collection. It's used to measure relative performance.
    for (col, repeat) in &collection_types.map(|col| match col {
        Collection::IterableSet => (col, 600000),
        Collection::IterableMap => (col, 280000),
        Collection::UnorderedSet => (col, 280),
        Collection::UnorderedMap => (col, 260),
        _ => (col, 0),
    }) {
        let total_gas = account
            .call(&contract_id.clone(), "nth")
            .args_json((col, repeat, element_number - deleted_element_number))
            .max_gas()
            .transact()
            .await?
            .unwrap()
            .total_gas_burnt
            .as_gas();

        perform_asserts(total_gas, col);
    }

    Ok(())
}
