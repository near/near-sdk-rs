use near_sdk::json_types::U128;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract, DevNetwork, Worker};

pub type Balance = U128;

async fn init(
    worker: &Worker<impl DevNetwork>,
    initial_balance: Balance,
) -> anyhow::Result<(Contract, Account)> {
    let contract = worker.dev_deploy(include_bytes!("../res/lockable_fungible_token.wasm")).await?;

    let res = contract
        .call("new")
        .args_json((contract.id(), initial_balance))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let alice = contract
        .as_account()
        .create_subaccount("alice")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    assert!(res.is_success());

    Ok((contract, alice))
}

#[tokio::test]
async fn test_owner_initial_state() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let worker = near_workspaces::sandbox().await?;
    let (contract, _) = init(&worker, initial_balance).await?;

    let res = contract.call("get_total_supply").view().await?;

    assert_eq!(res.json::<Balance>()?, initial_balance);

    let res = contract.call("get_total_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<Balance>()?, initial_balance);

    let res = contract.call("get_unlocked_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<Balance>()?, initial_balance);

    let res =
        contract.call("get_allowance").args_json((contract.id(), contract.id())).view().await?;
    assert_eq!(res.json::<NearToken>()?, NearToken::from_near(0));

    let res = contract
        .call("get_locked_balance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?;
    assert_eq!(res.json::<NearToken>()?, NearToken::from_yoctonear(0));

    Ok(())
}

#[tokio::test]
async fn test_set_allowance() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let allowance_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, alice) = init(&worker, initial_balance).await?;

    let res = contract
        .call("set_allowance")
        .args_json((alice.id(), allowance_amount))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let root_allowance = contract
        .call("get_allowance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?
        .json::<NearToken>()?;
    let alice_allowance = contract
        .call("get_allowance")
        .args_json((contract.id(), alice.id()))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(root_allowance, NearToken::from_yoctonear(0));
    assert_eq!(alice_allowance, allowance_amount);

    Ok(())
}

#[tokio::test]
async fn test_fail_set_allowance_self() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let allowance_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call("set_allowance")
        .args_json((contract.id(), allowance_amount))
        .max_gas()
        .transact()
        .await?
        .into_result();
    assert!(format!("{:?}", res).contains("Can't set allowance for yourself"));

    Ok(())
}

#[tokio::test]
async fn test_lock_owner() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let lock_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _) = init(&worker, initial_balance).await?;

    let res =
        contract.call("lock").args_json((contract.id(), lock_amount)).max_gas().transact().await?;
    assert!(res.is_success());

    let res = contract.call("get_unlocked_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<Balance>()?, U128(initial_balance.0 - lock_amount.as_yoctonear()));

    let res =
        contract.call("get_allowance").args_json((contract.id(), contract.id())).view().await?;
    assert_eq!(res.json::<NearToken>()?, NearToken::from_yoctonear(0));

    let res = contract
        .call("get_locked_balance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?;
    assert_eq!(res.json::<NearToken>()?, lock_amount);

    Ok(())
}

#[tokio::test]
async fn test_fail_lock() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let worker = near_workspaces::sandbox().await?;
    let (contract, alice) = init(&worker, initial_balance).await?;

    let res = contract.call("lock").args_json((contract.id(), "0")).max_gas().transact().await;
    assert!(format!("{:?}", res).contains("Can't lock 0 tokens"));

    let res = contract
        .call("lock")
        .args_json((contract.id(), NearToken::from_near(10001)))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough unlocked balance"));

    let res = alice
        .call(contract.id(), "lock")
        .args_json((contract.id(), NearToken::from_near(10).as_yoctonear().to_string()))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough allowance"));

    Ok(())
}

#[tokio::test]
async fn test_unlock_owner() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let lock_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _) = init(&worker, initial_balance).await?;

    let res =
        contract.call("lock").args_json((contract.id(), lock_amount)).max_gas().transact().await?;
    assert!(res.is_success());

    let res = contract
        .call("unlock")
        .args_json((contract.id(), lock_amount))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let res = contract.call("get_unlocked_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<Balance>()?, initial_balance);

    let res =
        contract.call("get_allowance").args_json((contract.id(), contract.id())).view().await?;
    assert_eq!(res.json::<NearToken>()?, NearToken::from_yoctonear(0));

    let res = contract
        .call("get_locked_balance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?;
    assert_eq!(res.json::<NearToken>()?, NearToken::from_yoctonear(0));

    Ok(())
}

#[tokio::test]
async fn test_fail_unlock() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let worker = near_workspaces::sandbox().await?;
    let (contract, _) = init(&worker, initial_balance).await?;

    let res = contract.call("unlock").args_json((contract.id(), "0")).max_gas().transact().await;
    assert!(format!("{:?}", res).contains("Can't unlock 0 tokens"));

    let res = contract
        .call("unlock")
        .args_json((contract.id(), NearToken::from_near(1).as_yoctonear().to_string()))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough locked tokens"));

    Ok(())
}

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, alice) = init(&worker, initial_balance).await?;

    let res = contract
        .call("transfer")
        .args_json((alice.id(), transfer_amount))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call("get_unlocked_balance")
        .args_json((contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    let alice_balance = contract
        .call("get_unlocked_balance")
        .args_json((alice.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(initial_balance.0 - transfer_amount.as_yoctonear(), root_balance.as_yoctonear());
    assert_eq!(transfer_amount.as_yoctonear(), alice_balance.as_yoctonear());

    Ok(())
}

#[tokio::test]
async fn test_fail_transfer() -> anyhow::Result<()> {
    let initial_balance: Balance = U128(NearToken::from_near(10000).as_yoctonear());
    let worker = near_workspaces::sandbox().await?;
    let (contract, alice) = init(&worker, initial_balance).await?;

    let res = contract.call("transfer").args_json((alice.id(), "0")).max_gas().transact().await;
    assert!(format!("{:?}", res).contains("Can't transfer 0 tokens"));

    let res = contract
        .call("transfer")
        .args_json((alice.id(), NearToken::from_near(10001)))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough unlocked balance"));

    Ok(())
}
