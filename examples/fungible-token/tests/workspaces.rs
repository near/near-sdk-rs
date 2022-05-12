
use near_sdk::json_types::U128;
use near_sdk::ONE_YOCTO;
use near_units::parse_near;
use workspaces::prelude::*;
use workspaces::{Account, AccountId, Contract, DevNetwork, Network, Worker};

async fn register_user(
    worker: &Worker<impl Network>,
    contract: &Contract,
    account_id: &AccountId,
) -> anyhow::Result<()> {
    let res = contract
        .call(&worker, "storage_deposit")
        .args_json((account_id, Option::<bool>::None))?
        .gas(300_000_000_000_000)
        .deposit(near_sdk::env::storage_byte_cost() * 125)
        .transact()
        .await?;
    assert!(res.is_success());

    Ok(())
}

async fn init(
    worker: &Worker<impl DevNetwork>,
    initial_balance: U128,
) -> anyhow::Result<(Contract, Account, Contract)> {
    let ft_contract =
        worker.dev_deploy(&include_bytes!("../res/fungible_token.wasm").to_vec()).await?;

    let res = ft_contract
        .call(&worker, "new_default_meta")
        .args_json((ft_contract.id(), initial_balance))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(res.is_success());

    let defi_contract = worker.dev_deploy(&include_bytes!("../res/defi.wasm").to_vec()).await?;

    let res = defi_contract
        .call(&worker, "new")
        .args_json((ft_contract.id(),))?
        .gas(300_000_000_000_000)
        .transact()
        .await?;
    assert!(res.is_success());

    let alice = ft_contract
        .as_account()
        .create_subaccount(&worker, "alice")
        .initial_balance(parse_near!("10 N"))
        .transact()
        .await?
        .into_result()?;
    register_user(worker, &ft_contract, alice.id()).await?;

    let res = ft_contract
        .call(&worker, "storage_deposit")
        .args_json((alice.id(), Option::<bool>::None))?
        .gas(300_000_000_000_000)
        .deposit(near_sdk::env::storage_byte_cost() * 125)
        .transact()
        .await?;
    assert!(res.is_success());

    return Ok((ft_contract, alice, defi_contract));
}

#[tokio::test]
async fn test_total_supply() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract.call(&worker, "ft_total_supply").view().await?;
    assert_eq!(res.json::<U128>()?, initial_balance);

    Ok(())
}

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let transfer_amount = U128::from(parse_near!("100 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, alice, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call(&worker, "ft_transfer")
        .args_json((alice.id(), transfer_amount, Option::<bool>::None))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    let alice_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((alice.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0 - transfer_amount.0, root_balance.0);
    assert_eq!(transfer_amount.0, alice_balance.0);

    Ok(())
}

#[tokio::test]
async fn test_close_account_empty_balance() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, alice, _) = init(&worker, initial_balance).await?;

    let res = alice
        .call(&worker, contract.id(), "storage_unregister")
        .args_json((Option::<bool>::None,))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.json::<bool>()?);

    Ok(())
}

#[tokio::test]
async fn test_close_account_non_empty_balance() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call(&worker, "storage_unregister")
        .args_json((Option::<bool>::None,))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await;
    assert!(format!("{:?}", res)
        .contains("Can't unregister the account with the positive balance without force"));

    let res = contract
        .call(&worker, "storage_unregister")
        .args_json((Some(false),))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await;
    assert!(format!("{:?}", res)
        .contains("Can't unregister the account with the positive balance without force"));

    Ok(())
}

#[tokio::test]
async fn simulate_close_account_force_non_empty_balance() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call(&worker, "storage_unregister")
        .args_json((Some(true),))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let res = contract.call(&worker, "ft_total_supply").view().await?;
    assert_eq!(res.json::<U128>()?.0, 0);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_burned_amount() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let transfer_amount = U128::from(parse_near!("100 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&worker, &contract, defi_contract.id()).await?;

    // root invests in defi by calling `ft_transfer_call`
    // TODO: Put two actions below into a batched transaction once workspaces supports them
    let res = contract
        .call(&worker, "ft_transfer_call")
        .args_json((defi_contract.id(), transfer_amount, Option::<String>::None, "10"))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());
    let res = contract
        .call(&worker, "storage_unregister")
        .args_json((Some(true),))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());
    assert!(res.json::<bool>()?);

    // TODO: Check callbacks once workspaces starts exposing them

    // let callback_outcome = outcome.get_receipt_results().remove(1).unwrap();
    //
    // assert_eq!(callback_outcome.logs()[0], "The account of the sender was deleted");
    // assert_eq!(callback_outcome.logs()[1], format!("Account @{} burned {}", root.account_id(), 10));
    //
    // let used_amount: U128 = callback_outcome.unwrap_json();
    // // Sender deleted the account. Even though the returned amount was 10, it was not refunded back
    // // to the sender, but was taken out of the receiver's balance and was burned.
    // assert_eq!(used_amount.0, transfer_amount);

    let res = contract.call(&worker, "ft_total_supply").view().await?;
    assert_eq!(res.json::<U128>()?.0, transfer_amount.0 - 10);
    let defi_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((defi_contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(defi_balance.0, transfer_amount.0 - 10);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_immediate_return_and_no_refund() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let transfer_amount = U128::from(parse_near!("100 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&worker, &contract, defi_contract.id()).await?;

    // root invests in defi by calling `ft_transfer_call`
    let res = contract
        .call(&worker, "ft_transfer_call")
        .args_json((defi_contract.id(), transfer_amount, Option::<String>::None, "take-my-money"))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    let defi_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((defi_contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0 - transfer_amount.0, root_balance.0);
    assert_eq!(transfer_amount.0, defi_balance.0);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_when_called_contract_not_registered_with_ft() -> anyhow::Result<()>
{
    let initial_balance = U128::from(parse_near!("10000 N"));
    let transfer_amount = U128::from(parse_near!("100 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // call fails because DEFI contract is not registered as FT user
    let res = contract
        .call(&worker, "ft_transfer_call")
        .args_json((defi_contract.id(), transfer_amount, Option::<String>::None, "take-my-money"))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await;
    assert!(res.is_err());

    // balances remain unchanged
    let root_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    let defi_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((defi_contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0, root_balance.0);
    assert_eq!(0, defi_balance.0);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_promise_and_refund() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let refund_amount = U128::from(parse_near!("50 N"));
    let transfer_amount = U128::from(parse_near!("100 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&worker, &contract, defi_contract.id()).await?;

    let res = contract
        .call(&worker, "ft_transfer_call")
        .args_json((
            defi_contract.id(),
            transfer_amount,
            Option::<String>::None,
            refund_amount.0.to_string(),
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    let defi_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((defi_contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0 - transfer_amount.0 + refund_amount.0, root_balance.0);
    assert_eq!(transfer_amount.0 - refund_amount.0, defi_balance.0);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_promise_panics_for_a_full_refund() -> anyhow::Result<()> {
    let initial_balance = U128::from(parse_near!("10000 N"));
    let transfer_amount = U128::from(parse_near!("100 N"));
    let worker = workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&worker, &contract, defi_contract.id()).await?;

    // root invests in defi by calling `ft_transfer_call`
    let res = contract
        .call(&worker, "ft_transfer_call")
        .args_json((
            defi_contract.id(),
            transfer_amount,
            Option::<String>::None,
            "no parsey as integer big panic oh no".to_string(),
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // TODO: Check promise errors once workspaces starts exposing them

    // assert_eq!(res.promise_errors().len(), 1);
    //
    // if let ExecutionStatus::Failure(execution_error) =
    //     &res.promise_errors().remove(0).unwrap().outcome().status
    // {
    //     assert!(execution_error.to_string().contains("ParseIntError"));
    // } else {
    //     unreachable!();
    // }

    // balances remain unchanged
    let root_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    let defi_balance = contract
        .call(&worker, "ft_balance_of")
        .args_json((defi_contract.id(),))?
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance, root_balance);
    assert_eq!(0, defi_balance.0);

    Ok(())
}
