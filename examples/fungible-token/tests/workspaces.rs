use near_workspaces::operations::Function;
use near_workspaces::result::ValueOrReceiptId;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, AccountId, Contract, DevNetwork, Worker};

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
async fn register_user(contract: &Contract, account_id: &AccountId) -> anyhow::Result<()> {
    let res = contract
        .call("storage_deposit")
        .args_json((account_id, Option::<bool>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(near_sdk::env::storage_byte_cost() * 125))
        .transact()
        .await?;
    assert!(res.is_success());

    Ok(())
}

async fn init(
    worker: &Worker<impl DevNetwork>,
    initial_balance: NearToken,
) -> anyhow::Result<(Contract, Account, Contract)> {
    let ft_contract = worker.dev_deploy(include_bytes!("../res/fungible_token.wasm")).await?;

    let res = ft_contract
        .call("new_default_meta")
        .args_json((ft_contract.id(), initial_balance))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    let defi_contract = worker.dev_deploy(include_bytes!("../res/defi.wasm")).await?;

    let res = defi_contract.call("new").args_json((ft_contract.id(),)).max_gas().transact().await?;
    assert!(res.is_success());

    let alice = ft_contract
        .as_account()
        .create_subaccount("alice")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    register_user(&ft_contract, alice.id()).await?;

    let res = ft_contract
        .call("storage_deposit")
        .args_json((alice.id(), Option::<bool>::None))
        .deposit(NearToken::from_yoctonear(near_sdk::env::storage_byte_cost() * 125))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    return Ok((ft_contract, alice, defi_contract));
}

#[tokio::test]
async fn test_total_supply() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract.call("ft_total_supply").view().await?;
    assert_eq!(res.json::<NearToken>()?, initial_balance);

    Ok(())
}

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, alice, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call("ft_transfer")
        .args_json((alice.id(), transfer_amount, Option::<bool>::None))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call("ft_balance_of")
        .args_json((contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    let alice_balance = contract
        .call("ft_balance_of")
        .args_json((alice.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(
        initial_balance.as_yoctonear() - transfer_amount.as_yoctonear(),
        root_balance.as_yoctonear()
    );
    assert_eq!(transfer_amount, alice_balance);

    Ok(())
}

#[tokio::test]
async fn test_close_account_empty_balance() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let worker = near_workspaces::sandbox().await?;
    let (contract, alice, _) = init(&worker, initial_balance).await?;

    let res = alice
        .call(contract.id(), "storage_unregister")
        .args_json((Option::<bool>::None,))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.json::<bool>()?);

    Ok(())
}

#[tokio::test]
async fn test_close_account_non_empty_balance() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call("storage_unregister")
        .args_json((Option::<bool>::None,))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await;
    assert!(format!("{:?}", res)
        .contains("Can't unregister the account with the positive balance without force"));

    let res = contract
        .call("storage_unregister")
        .args_json((Some(false),))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await;
    assert!(format!("{:?}", res)
        .contains("Can't unregister the account with the positive balance without force"));

    Ok(())
}

#[tokio::test]
async fn simulate_close_account_force_non_empty_balance() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract
        .call("storage_unregister")
        .args_json((Some(true),))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let res = contract.call("ft_total_supply").view().await?;
    assert_eq!(res.json::<NearToken>()?, NearToken::from_near(0));

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_burned_amount() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&contract, defi_contract.id()).await?;

    // root invests in defi by calling `ft_transfer_call`
    let res = contract
        .batch()
        .call(
            Function::new("ft_transfer_call")
                .args_json((defi_contract.id(), transfer_amount, Option::<String>::None, "10"))
                .deposit(ONE_YOCTO)
                .gas(near_sdk::Gas::from_tgas(150)),
        )
        .call(
            Function::new("storage_unregister")
                .args_json((Some(true),))
                .deposit(ONE_YOCTO)
                .gas(near_sdk::Gas::from_tgas(150)),
        )
        .transact()
        .await?;
    assert!(res.is_success());

    let logs = res.logs();
    let expected = format!("Account @{} burned {}", contract.id(), 10);
    assert!(logs.len() >= 2);
    assert!(logs.contains(&"The account of the sender was deleted"));
    assert!(logs.contains(&(expected.as_str())));

    match res.receipt_outcomes()[5].clone().into_result()? {
        ValueOrReceiptId::Value(val) => {
            let used_amount = val.json::<NearToken>()?;
            assert_eq!(used_amount, transfer_amount);
        }
        _ => panic!("Unexpected receipt id"),
    }
    assert!(res.json::<bool>()?);

    let res = contract.call("ft_total_supply").view().await?;
    assert_eq!(res.json::<NearToken>()?, transfer_amount.saturating_sub(NearToken::from_yoctonear(10)));
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(defi_balance.as_yoctonear(), transfer_amount.as_yoctonear() - 10);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_immediate_return_and_no_refund() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&contract, defi_contract.id()).await?;

    // root invests in defi by calling `ft_transfer_call`
    let res = contract
        .call("ft_transfer_call")
        .args_json((defi_contract.id(), transfer_amount, Option::<String>::None, "take-my-money"))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call("ft_balance_of")
        .args_json((contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(
        initial_balance.saturating_sub(transfer_amount),
        root_balance
    );
    assert_eq!(transfer_amount, defi_balance);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_when_called_contract_not_registered_with_ft() -> anyhow::Result<()>
{
    let initial_balance = NearToken::from_near(10000);
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // call fails because DEFI contract is not registered as FT user
    let res = contract
        .call("ft_transfer_call")
        .args_json((defi_contract.id(), transfer_amount, Option::<String>::None, "take-my-money"))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_failure());

    // balances remain unchanged
    let root_balance = contract
        .call("ft_balance_of")
        .args_json((contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(initial_balance, root_balance);
    assert_eq!(NearToken::from_near(0), defi_balance);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_promise_and_refund() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let refund_amount = NearToken::from_near(50);
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&contract, defi_contract.id()).await?;

    let res = contract
        .call("ft_transfer_call")
        .args_json((
            defi_contract.id(),
            transfer_amount.as_yoctonear().to_string(),
            Option::<String>::None,
            refund_amount.as_yoctonear().to_string(),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance = contract
        .call("ft_balance_of")
        .args_json((contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(
        initial_balance.as_yoctonear() - transfer_amount.as_yoctonear()
            + refund_amount.as_yoctonear(),
        root_balance.as_yoctonear()
    );
    assert_eq!(
        transfer_amount.as_yoctonear() - refund_amount.as_yoctonear(),
        defi_balance.as_yoctonear()
    );

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_promise_panics_for_a_full_refund() -> anyhow::Result<()> {
    let initial_balance = NearToken::from_near(10000);
    let transfer_amount = NearToken::from_near(100);
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;

    // defi contract must be registered as a FT account
    register_user(&contract, defi_contract.id()).await?;

    // root invests in defi by calling `ft_transfer_call`
    let res = contract
        .call("ft_transfer_call")
        .args_json((
            defi_contract.id(),
            transfer_amount,
            Option::<String>::None,
            "no parsey as integer big panic oh no".to_string(),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let promise_failures = res.receipt_failures();
    assert_eq!(promise_failures.len(), 1);
    let failure = promise_failures[0].clone().into_result();
    if let Err(err) = failure {
        assert!(format!("{:?}", err).contains("ParseIntError"));
    } else {
        unreachable!();
    }

    // balances remain unchanged
    let root_balance = contract
        .call("ft_balance_of")
        .args_json((contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<NearToken>()?;
    assert_eq!(initial_balance, root_balance);
    assert_eq!(0, defi_balance.as_yoctonear());

    Ok(())
}
