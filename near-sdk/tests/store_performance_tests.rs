use near_account_id::AccountId;
use near_gas::NearGas;
use near_workspaces::network::Sandbox;
use near_workspaces::types::{KeyType, SecretKey};
use near_workspaces::{Account, Worker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
use tokio::task::JoinSet;

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
    Insert(u32),
    Remove(u32),
    Flush,
    Contains(u32),
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
#[tokio::test]
#[allow(clippy::needless_range_loop)]
// Note that different types of tests are executed sequentially, as the previous tests are populating
// the data.
// Also, the tests for each collection are executed sequentially, as otherwise near-sandbox doesn't cope.
async fn combined_test() -> anyhow::Result<()> {
    let worker = Arc::new(near_workspaces::sandbox().await?);
    let contract = worker.dev_deploy(include_bytes!("test-contracts/store/res/store.wasm")).await?;
    let res = contract.call("new").max_gas().transact().await?;
    assert!(res.is_success());
    let contract_id = contract.id().clone();
    let mut account_pool = HashMap::new();

    // Generate different accounts to avoid Nonce collisions when executing transactions in parallel.
    let mut account_set = JoinSet::new();
    for col in Collection::iter() {
        account_pool.insert(col, Vec::new());
        for val in 0..=17 {
            account_set.spawn(dev_generate(worker.clone(), col, val.to_string()));
        }
    }

    while let Some(account) = account_set.join_next().await {
        let (account, col) = account??;
        account_pool.get_mut(&col).unwrap().push(account);
    }

    // insert
    for (col, max_iterations) in Collection::iter().map(|col| match col {
        // TreeMap performance is inferior to other collections.
        Collection::TreeMap => (col, 15),
        _ => (col, 16),
    }) {
        let collection_account_pool = account_pool.get(&col).unwrap().clone();
        let contract_id = contract_id.clone();
        let mut total_gas: u64 = 0;
        let mut futures = JoinSet::new();

        for val in 0..max_iterations {
            let account: Account = collection_account_pool[val].clone();
            let txn = account
                .call(&contract_id.clone(), "exec")
                .args_json((col, Op::Insert(val as u32)))
                .transact();
            futures.spawn(txn);
        }
        while let Some(res) = futures.join_next().await {
            total_gas += res??.unwrap().total_gas_burnt.as_gas();
        }
        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression: {}",
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed: {}, adjust the number of iterations to spot regressions",
            NearGas::from_gas(total_gas)
        );
    }

    // iter
    for (col, max_iterations) in Collection::iter()
        .filter(|col| {
            // Those collections are not iterable.
            !matches!(col, Collection::LookupMap) && !matches!(col, Collection::LookupSet)
        })
        .map(|col| match col {
            // *Map performance is inferior to other collections.
            Collection::IterableMap | Collection::UnorderedMap | Collection::TreeMap => (col, 12),
            _ => (col, 14),
        })
    {
        let collection_account_pool = account_pool.get(&col).unwrap().clone();
        let contract_id = contract_id.clone();
        let mut total_gas: u64 = 0;
        let mut futures = JoinSet::new();

        for val in 0..max_iterations {
            let account: Account = collection_account_pool[val].clone();
            let txn = account
                .call(&contract_id.clone(), "exec")
                .args_json((col, Op::Iter(15)))
                .max_gas()
                .transact();
            futures.spawn(txn);
        }
        while let Some(res) = futures.join_next().await {
            total_gas += res??.unwrap().total_gas_burnt.as_gas();
        }

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression: {}",
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed: {}, adjust the number of iterations to spot regressions",
            NearGas::from_gas(total_gas)
        );
    }

    // contains
    for col in Collection::iter().filter(|col| {
        // No `contains` in vector.
        !matches!(col, Collection::Vector)
    }) {
        let collection_account_pool = account_pool.get(&col).unwrap().clone();
        let contract_id = contract_id.clone();
        let mut total_gas: u64 = 0;
        let mut futures = JoinSet::new();

        for val in 0..17 {
            let account: Account = collection_account_pool[val].clone();
            let txn = account
                .call(&contract_id.clone(), "exec")
                .args_json((col, Op::Contains(3)))
                .max_gas()
                .transact();
            futures.spawn(txn);
        }
        while let Some(res) = futures.join_next().await {
            total_gas += res??.unwrap().total_gas_burnt.as_gas();
        }

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression: {}",
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed: {}, adjust the number of iterations to spot regressions",
            NearGas::from_gas(total_gas)
        );
    }

    // flush
    for col in Collection::iter().filter(|col| {
        // LookupSet is not flushable.
        !matches!(col, Collection::LookupSet)
    }) {
        let collection_account_pool = account_pool.get(&col).unwrap().clone();
        let contract_id = contract_id.clone();
        let mut total_gas: u64 = 0;
        let mut futures = JoinSet::new();

        for val in 0..17 {
            let account: Account = collection_account_pool[val].clone();
            let txn = account
                .call(&contract_id.clone(), "exec")
                .args_json((col, Op::Flush))
                .max_gas()
                .transact();
            futures.spawn(txn);
        }
        while let Some(res) = futures.join_next().await {
            total_gas += res??.unwrap().total_gas_burnt.as_gas();
        }

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression: {}",
            NearGas::from_gas(total_gas)
        );
        assert!(
            total_gas > NearGas::from_tgas(90).as_gas(),
            "not enough gas consumed: {}, adjust the number of iterations to spot regressions",
            NearGas::from_gas(total_gas)
        );
    }

    // remove
    for col in Collection::iter() {
        let collection_account_pool = account_pool.get(&col).unwrap().clone();
        let contract_id = contract_id.clone();
        let mut total_gas: u64 = 0;
        let mut futures = JoinSet::new();

        // Can't use more than 15, because that's how much was inserted.
        for val in 0..15 {
            let account: Account = collection_account_pool[val].clone();
            let txn = account
                .call(&contract_id.clone(), "exec")
                .args_json((col, Op::Remove(val as u32)))
                .max_gas()
                .transact();
            futures.spawn(txn);
        }
        while let Some(res) = futures.join_next().await {
            total_gas += res??.unwrap().total_gas_burnt.as_gas();
        }

        assert!(
            total_gas < NearGas::from_tgas(100).as_gas(),
            "performance regression: {}",
            NearGas::from_gas(total_gas)
        );
        // A slight reduction of the lower bound here as we have nothing else to remove.
        assert!(
            total_gas > NearGas::from_tgas(87).as_gas(),
            "not enough gas consumed: {}, adjust the number of iterations to spot regressions",
            NearGas::from_gas(total_gas)
        );
    }

    Ok(())
}
