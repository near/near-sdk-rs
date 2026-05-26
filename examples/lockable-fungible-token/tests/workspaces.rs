use std::str::FromStr;

use near_sdk::json_types::U128;
use near_workspaces::cargo_near_build;
use near_workspaces::{Account, Contract, types::NearToken};
use rstest::{fixture, rstest};

#[fixture]
fn initial_balance() -> U128 {
    U128::from(NearToken::from_near(10000).as_yoctonear())
}

fn build_contract(path: &str, contract_name: &str) -> Vec<u8> {
    let artifact = cargo_near_build::build_with_cli(cargo_near_build::BuildOpts {
        manifest_path: Some(
            cargo_near_build::camino::Utf8PathBuf::from_str(path).expect("camino PathBuf from str"),
        ),
        ..Default::default()
    })
    .expect(&format!("building `{}` contract for tests", contract_name));

    let contract_wasm = std::fs::read(&artifact)
        .map_err(|err| format!("accessing {} to read wasm contents: {}", artifact, err))
        .expect("std::fs::read");
    contract_wasm
}

#[fixture]
#[once]
fn lockable_fungible_contract_wasm() -> Vec<u8> {
    build_contract("./Cargo.toml", "lockable-fungible-token")
}

#[fixture]
async fn initialized_contract(
    initial_balance: U128,
    lockable_fungible_contract_wasm: &Vec<u8>,
) -> anyhow::Result<(Contract, Account)> {
    let worker = near_workspaces::sandbox().await?;

    let contract = worker.dev_deploy(lockable_fungible_contract_wasm).await?;

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

#[rstest]
#[rstest]
#[tokio::test]
async fn test_owner_initial_state(
    initial_balance: U128,
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let (contract, _) = initialized_contract.await?;

    let res = contract.call("get_total_supply").view().await?;
    assert_eq!(res.json::<U128>()?, initial_balance);

    let res = contract.call("get_total_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<U128>()?, initial_balance);

    let res = contract.call("get_unlocked_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<U128>()?, initial_balance);

    let res =
        contract.call("get_allowance").args_json((contract.id(), contract.id())).view().await?;
    assert_eq!(res.json::<U128>()?, U128::from(0));

    let res = contract
        .call("get_locked_balance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?;
    assert_eq!(res.json::<U128>()?, U128::from(0));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_set_allowance(
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let allowance_amount = U128::from(NearToken::from_near(100).as_yoctonear());

    let (contract, alice) = initialized_contract.await?;

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
        .json::<U128>()?;
    let alice_allowance = contract
        .call("get_allowance")
        .args_json((contract.id(), alice.id()))
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(root_allowance, U128::from(0));
    assert_eq!(alice_allowance, allowance_amount);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_fail_set_allowance_self(
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let allowance_amount = U128::from(NearToken::from_near(100).as_yoctonear());

    let (contract, _) = initialized_contract.await?;

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

#[rstest]
#[tokio::test]
async fn test_lock_owner(
    initial_balance: U128,
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let lock_amount = U128::from(NearToken::from_near(100).as_yoctonear());

    let (contract, _) = initialized_contract.await?;

    let res =
        contract.call("lock").args_json((contract.id(), lock_amount)).max_gas().transact().await?;
    assert!(res.is_success());

    let res = contract.call("get_unlocked_balance").args_json((contract.id(),)).view().await?;
    assert_eq!(res.json::<U128>()?.0, initial_balance.0 - lock_amount.0);

    let res =
        contract.call("get_allowance").args_json((contract.id(), contract.id())).view().await?;
    assert_eq!(res.json::<U128>()?, U128::from(0));

    let res = contract
        .call("get_locked_balance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?;
    assert_eq!(res.json::<U128>()?, lock_amount);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_fail_lock(
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let (contract, alice) = initialized_contract.await?;

    let res = contract.call("lock").args_json((contract.id(), "0")).max_gas().transact().await;
    assert!(format!("{:?}", res).contains("Can't lock 0 tokens"));

    let res = contract
        .call("lock")
        .args_json((contract.id(), U128::from(NearToken::from_near(10001).as_yoctonear())))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough unlocked balance"));

    let res = alice
        .call(contract.id(), "lock")
        .args_json((contract.id(), U128::from(NearToken::from_near(10).as_yoctonear())))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough allowance"));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_unlock_owner(
    initial_balance: U128,
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let lock_amount = U128::from(NearToken::from_near(100).as_yoctonear());

    let (contract, _) = initialized_contract.await?;

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
    assert_eq!(res.json::<U128>()?.0, initial_balance.0);

    let res =
        contract.call("get_allowance").args_json((contract.id(), contract.id())).view().await?;
    assert_eq!(res.json::<U128>()?, U128::from(0));

    let res = contract
        .call("get_locked_balance")
        .args_json((contract.id(), contract.id()))
        .view()
        .await?;
    assert_eq!(res.json::<U128>()?, U128::from(0));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_fail_unlock(
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let (contract, _) = initialized_contract.await?;

    let res = contract.call("unlock").args_json((contract.id(), "0")).max_gas().transact().await;
    assert!(format!("{:?}", res).contains("Can't unlock 0 tokens"));

    let res = contract
        .call("unlock")
        .args_json((contract.id(), U128::from(NearToken::from_near(1).as_yoctonear())))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough locked tokens"));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_simple_transfer(
    initial_balance: U128,
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());

    let (contract, alice) = initialized_contract.await?;

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
        .json::<U128>()?;
    let alice_balance = contract
        .call("get_unlocked_balance")
        .args_json((alice.id(),))
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0 - transfer_amount.0, root_balance.0);
    assert_eq!(transfer_amount.0, alice_balance.0);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_fail_transfer(
    #[future] initialized_contract: anyhow::Result<(Contract, Account)>,
) -> anyhow::Result<()> {
    let (contract, alice) = initialized_contract.await?;

    let res = contract.call("transfer").args_json((alice.id(), "0")).max_gas().transact().await;
    assert!(format!("{:?}", res).contains("Can't transfer 0 tokens"));

    let res = contract
        .call("transfer")
        .args_json((alice.id(), U128::from(NearToken::from_near(10001).as_yoctonear())))
        .max_gas()
        .transact()
        .await;
    assert!(format!("{:?}", res).contains("Not enough unlocked balance"));

    Ok(())
}
