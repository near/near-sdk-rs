// As wasm VM performance is tested, there is no need to test this on other types of OS.
// This test runs only on Linux, as it's much slower on OS X due to an interpreted VM.
// #![cfg(target_os = "linux")]

use near_account_id::AccountId;
use near_gas::NearGas;
use near_workspaces::network::Sandbox;
use near_workspaces::types::{KeyType, SecretKey};
use near_workspaces::{Account, Worker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

const DEFAULT_INDEX_OFFSET: usize = 0;

#[derive(Serialize, Deserialize, EnumIter, Display, Copy, Clone, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum Op {
    Insert(usize),
    Remove,
    Flush,
    Contains(usize),
    Iter(usize),
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
    let contract = worker.dev_deploy(include_bytes!("test-contracts/store/res/store.wasm")).await?;
    let res = contract.call("new").max_gas().transact().await?;
    assert!(res.is_success());
    Ok((worker, contract.id().clone()))
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
    let (account, contract_id) = setup().await?;
    // insert
    for (col, max_iterations) in Collection::iter().map(|col| match col {
        Collection::TreeMap => (col, 310),
        Collection::IterableSet => (col, 375),
        Collection::IterableMap => (col, 360),
        Collection::UnorderedSet => (col, 340),
        Collection::UnorderedMap => (col, 350),
        Collection::LookupMap => (col, 600),
        Collection::LookupSet => (col, 970),
        Collection::Vector => (col, 1000),
    }) {
        let txn = account
            .call(&contract_id, "exec")
            .args_json((col, Op::Insert(DEFAULT_INDEX_OFFSET), max_iterations))
            .max_gas()
            .transact()
            .await;

        let res = txn?;
        let total_gas = res.unwrap().total_gas_burnt.as_gas();

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression {}: {}",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed {}: {}, adjust the number of iterations to spot regressions",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
    }

    // remove
    for (col, max_iterations) in Collection::iter().map(|col| match col {
        Collection::TreeMap => (col, 220),
        Collection::IterableSet => (col, 120),
        Collection::IterableMap => (col, 115),
        Collection::UnorderedSet => (col, 220),
        Collection::UnorderedMap => (col, 220),
        Collection::LookupMap => (col, 480),
        Collection::LookupSet => (col, 970),
        Collection::Vector => (col, 500),
    }) {
        let txn = account
            .call(&contract_id, "exec")
            .args_json((col, Op::Remove, max_iterations))
            .max_gas()
            .transact()
            .await;

        let res = txn?;
        let total_gas = res.unwrap().total_gas_burnt.as_gas();

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression {}: {}",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed {}: {}, adjust the number of iterations to spot regressions",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
    }

    Ok(())
}

#[tokio::test]
#[allow(clippy::needless_range_loop)]
async fn iter() -> anyhow::Result<()> {
    let element_number = 300;
    let (account, contract_id) = setup().await?;
    // prepopulate
    for col in Collection::iter() {
        let txn = account
            .call(&contract_id, "exec")
            .args_json((col, Op::Insert(DEFAULT_INDEX_OFFSET), element_number))
            .max_gas()
            .transact()
            .await;

        let res = txn?;
        let _ = res.unwrap();
    }

    // iter
    for (col, repeat) in Collection::iter()
        .filter(|col| !matches!(col, Collection::LookupMap | Collection::LookupSet))
        .map(|col| match col {
            Collection::TreeMap => (col, 84),
            Collection::IterableSet => (col, 450),
            Collection::IterableMap => (col, 140),
            Collection::UnorderedSet => (col, 450),
            Collection::UnorderedMap => (col, 140),
            Collection::Vector => (col, 450),
            _ => (col, 0),
        })
    {
        let txn = account
            .call(&contract_id.clone(), "exec")
            .args_json((col, Op::Iter(repeat), element_number))
            .max_gas()
            .transact()
            .await;

        let res = txn?;
        let total_gas = res.unwrap().total_gas_burnt.as_gas();

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression {}: {}",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed {}: {}, adjust the number of iterations to spot regressions",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
    }

    Ok(())
}

#[tokio::test]
async fn contains() -> anyhow::Result<()> {
    let element_number = 200;
    let (account, contract_id) = setup().await?;
    // prepopulate
    for col in Collection::iter() {
        let txn = account
            .call(&contract_id, "exec")
            .args_json((col, Op::Insert(DEFAULT_INDEX_OFFSET), element_number))
            .max_gas()
            .transact()
            .await;

        let res = txn?;
        let _ = res.unwrap();
    }

    // contains
    for (col, repeat) in
        Collection::iter().filter(|col| !matches!(col, Collection::Vector)).map(|col| match col {
            Collection::TreeMap => (col, 6),
            Collection::IterableSet => (col, 6),
            Collection::IterableMap => (col, 6),
            Collection::UnorderedSet => (col, 6),
            Collection::UnorderedMap => (col, 6),
            Collection::LookupMap => (col, 8),
            Collection::LookupSet => (col, 7),
            _ => (col, 0),
        })
    {
        let txn = account
            .call(&contract_id.clone(), "exec")
            .args_json((col, Op::Contains(repeat), element_number))
            .max_gas()
            .transact()
            .await;

        let res = txn?;
        let total_gas = res.unwrap().total_gas_burnt.as_gas();

        // Slightly modified to fit the max gas consumption.
        assert!(
            total_gas < NearGas::from_tgas(103).as_gas(),
            "performance regression {}: {}",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed {}: {}, adjust the number of iterations to spot regressions",
            col.clone(),
            NearGas::from_gas(total_gas)
        );
    }

    Ok(())
}
