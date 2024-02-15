use near_sdk::borsh::de;
use near_sdk::json_types::U128;
use near_workspaces::operations::Function;
use near_workspaces::result::ValueOrReceiptId;
use near_workspaces::{types::NearToken, Account, AccountId, Contract, DevNetwork, Worker};

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

async fn register_user(contract: &Contract, account_id: &AccountId) -> anyhow::Result<()> {
    let res = contract
        .call("storage_deposit")
        .args_json((account_id, Option::<bool>::None))
        .max_gas()
        .deposit(near_sdk::env::storage_byte_cost().saturating_mul(125))
        .transact()
        .await?;
    assert!(res.is_success());

    Ok(())
}

async fn init(
    worker: &Worker<impl DevNetwork>,
    initial_balance: U128,
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
        .deposit(near_sdk::env::storage_byte_cost().saturating_mul(125))
        .max_gas()
        .transact()
        .await?;
    assert!(res.is_success());

    return Ok((ft_contract, alice, defi_contract));
}

#[tokio::test]
async fn test_total_supply() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, _) = init(&worker, initial_balance).await?;

    let res = contract.call("ft_total_supply").view().await?;
    assert_eq!(res.json::<U128>()?, initial_balance);

    Ok(())
}

#[tokio::test]
async fn test_simple_transfer() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());
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

    let root_balance =
        contract.call("ft_balance_of").args_json((contract.id(),)).view().await?.json::<U128>()?;
    let alice_balance =
        contract.call("ft_balance_of").args_json((alice.id(),)).view().await?.json::<U128>()?;
    assert_eq!(initial_balance.0 - transfer_amount.0, root_balance.0);
    assert_eq!(transfer_amount.0, alice_balance.0);

    Ok(())
}

#[tokio::test]
async fn test_close_account_empty_balance() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
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
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
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
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
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
    assert_eq!(res.json::<U128>()?.0, 0);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_burned_amount() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());
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
            let used_amount = val.json::<U128>()?;
            assert_eq!(used_amount, transfer_amount);
        }
        _ => panic!("Unexpected receipt id"),
    }
    assert!(res.json::<bool>()?);

    let res = contract.call("ft_total_supply").view().await?;
    assert_eq!(res.json::<U128>()?.0, transfer_amount.0 - 10);
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(defi_balance.0, transfer_amount.0 - 10);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_immediate_return_and_no_refund() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());
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

    let root_balance =
        contract.call("ft_balance_of").args_json((contract.id(),)).view().await?.json::<U128>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
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
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());
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
    let root_balance =
        contract.call("ft_balance_of").args_json((contract.id(),)).view().await?.json::<U128>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0, root_balance.0);
    assert_eq!(0, defi_balance.0);

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_with_promise_and_refund() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let refund_amount = U128::from(NearToken::from_near(50).as_yoctonear());
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());
    let worker = near_workspaces::sandbox().await?;
    let (contract, _, defi_contract) = init(&worker, initial_balance).await?;


    // defi contract must be registered as a FT account
    register_user(&contract, defi_contract.id()).await?;

 
    // let defi_before = defi_contract.view_account().await?.balance.as_yoctonear();
    let hacker = defi_contract
        .as_account()
        .create_subaccount("hacker")
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    let hacker_id = "hacker".to_string();

    // let hacker_id = hacker.id();
    let hacker_before = hacker.view_account().await?.balance.as_yoctonear();
    println!("Hacker: {}", hacker.id());

    let contract_before =  contract.view_account().await?.balance.as_yoctonear();
    let res = contract
        .call("storage_deposit")
        .args_json((hacker_id.clone(), Option::<bool>::None))
        .max_gas()
        .deposit(NearToken::from_near(10))
        .transact()
        .await?;
    assert!(res.is_success());
    let hacker_balance =
        contract.call("ft_balance_of").args_json((hacker_id,)).view().await?.json::<U128>()?;
    assert_eq!(0, hacker_balance.0);

    
    let res = contract
        .call("ft_transfer_call")
        .args_json((
            defi_contract.id(),
            transfer_amount,
            Option::<String>::None,
            refund_amount.0.to_string(),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    let root_balance =
        contract.call("ft_balance_of").args_json((contract.id(),)).view().await?.json::<U128>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance.0 - transfer_amount.0 + refund_amount.0, root_balance.0);
    assert_eq!(transfer_amount.0 - refund_amount.0, defi_balance.0);

    println!("{}", NearToken::from_yoctonear(contract_before - contract.view_account().await?.balance.as_yoctonear()));
    // println!("{}", NearToken::from_yoctonear(contract.view_account().await?.balance.as_yoctonear()-contract_before));

    assert!( contract_before > contract.view_account().await?.balance.as_yoctonear());
    
    assert!(hacker_before == hacker.view_account().await?.balance.as_yoctonear());

    println!("Account balance: {}", contract.view_account().await?.balance.as_yoctonear()); 

    println!("Account defi balance: {}",defi_contract.view_account().await?.balance.as_yoctonear()); 
    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_promise_panics_for_a_full_refund() -> anyhow::Result<()> {
    let initial_balance = U128::from(NearToken::from_near(10000).as_yoctonear());
    let transfer_amount = U128::from(NearToken::from_near(100).as_yoctonear());
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
    let root_balance =
        contract.call("ft_balance_of").args_json((contract.id(),)).view().await?.json::<U128>()?;
    let defi_balance = contract
        .call("ft_balance_of")
        .args_json((defi_contract.id(),))
        .view()
        .await?
        .json::<U128>()?;
    assert_eq!(initial_balance, root_balance);
    assert_eq!(0, defi_balance.0);

    Ok(())
}
