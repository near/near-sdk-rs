use near_kit::json_types::U128;
use near_kit::protocol::Action;
use near_kit::rpc::{ExecutionStatus, Finality};
use near_kit::transaction::{ExecutedOptimistic, FunctionCall};
use near_kit::{AccountId, Gas, NearToken, SigningNear};
use near_sandbox::Sandbox;
use serde_json::json;

use crate::shared::{
    ONE_TOKEN, ONE_YOCTO, TOTAL_SUPPLY, build_contract, failures, ft_balance_of, register,
};

/// How many tokens Alice sends to the DeFi contract in each test.
const AMOUNT: u128 = 100 * ONE_TOKEN;

/// Runs the shared setup and deploys the DeFi contract to `defi.alice`.
/// The DeFi contract isn't registered with the token contract yet.
async fn setup() -> anyhow::Result<(Sandbox, SigningNear<ExecutedOptimistic>, AccountId, AccountId)>
{
    let (node, alice, _, ft_contract_id) = crate::shared::setup().await?;

    // Create the DeFi account, fund it, deploy the DeFi contract and point it at the token
    let wasm = build_contract("test-contract-defi").await?;
    let defi_contract_id = alice.account_id().sub_account("defi")?;

    alice
        .transaction(&defi_contract_id)
        .add_action(Action::create_account())
        .add_action(Action::transfer(NearToken::from_near(10)))
        .add_action(Action::deploy_contract(wasm))
        .add_action(
            FunctionCall::new("new")
                .args(json!({ "fungible_token_account_id": ft_contract_id }))
                .gas(Gas::from_tgas(1)),
        )
        .await?
        .result()?;

    Ok((node, alice, ft_contract_id, defi_contract_id))
}

/// `ft_transfer_call` that sends `AMOUNT` tokens to `defi_contract_id` with `msg`.
///
/// The DeFi contract keeps all the tokens for `"take-my-money"`. Otherwise it parses `msg`
/// as the amount to return, in a cross-contract call to itself.
fn transfer_call(defi_contract_id: &AccountId, msg: &str) -> FunctionCall {
    FunctionCall::new("ft_transfer_call")
        .args(json!({
            "receiver_id": defi_contract_id,
            "amount": U128(AMOUNT),
            "msg": msg,
        }))
        .gas(Gas::from_tgas(15))
        .deposit(ONE_YOCTO)
}

#[tokio::test]
async fn test_transfer_call_with_immediate_return_and_no_refund() -> anyhow::Result<()> {
    let (_node, alice, ft_contract_id, defi_contract_id) = setup().await?;

    register(&alice, &ft_contract_id, &defi_contract_id).await?;

    // `ft_transfer_call` returns how many tokens the receiver used
    let used: U128 = alice
        .transaction(&ft_contract_id)
        .add_action(transfer_call(&defi_contract_id, "take-my-money"))
        .await?
        .json()?;

    assert_eq!(used.0, AMOUNT);

    let near = alice.near();

    assert_eq!(
        ft_balance_of(near, &ft_contract_id, alice.account_id()).await?,
        TOTAL_SUPPLY - AMOUNT
    );
    assert_eq!(ft_balance_of(near, &ft_contract_id, &defi_contract_id).await?, AMOUNT);

    Ok(())
}

#[tokio::test]
async fn test_transfer_call_with_promise_and_refund() -> anyhow::Result<()> {
    let (_node, alice, ft_contract_id, defi_contract_id) = setup().await?;
    let refund = 50 * ONE_TOKEN;

    register(&alice, &ft_contract_id, &defi_contract_id).await?;

    let used: U128 = alice
        .transaction(&ft_contract_id)
        .add_action(transfer_call(&defi_contract_id, &refund.to_string()))
        .await?
        .json()?;

    assert_eq!(used.0, AMOUNT - refund);

    let near = alice.near();

    assert_eq!(
        ft_balance_of(near, &ft_contract_id, alice.account_id()).await?,
        TOTAL_SUPPLY - AMOUNT + refund
    );
    assert_eq!(ft_balance_of(near, &ft_contract_id, &defi_contract_id).await?, AMOUNT - refund);

    Ok(())
}

#[tokio::test]
async fn test_transfer_call_promise_panics_for_a_full_refund() -> anyhow::Result<()> {
    let (_node, alice, ft_contract_id, defi_contract_id) = setup().await?;

    register(&alice, &ft_contract_id, &defi_contract_id).await?;

    // The DeFi contract panics on a `msg` that isn't a number, so all the tokens go back
    let tx_details = alice
        .transaction(&ft_contract_id)
        .add_action(transfer_call(&defi_contract_id, "no parsey as integer big panic oh no"))
        .await?;

    // An `expect` panic starts with its location in the source, so check only the end
    let failures = failures(&tx_details);

    assert!(
        matches!(
            failures.as_slice(),
            [failure] if failure.ends_with("Not an integer: ParseIntError { kind: InvalidDigit }")
        ),
        "{failures:?}"
    );

    let used: U128 = tx_details.json()?;

    assert_eq!(used.0, 0);

    let near = alice.near();

    assert_eq!(ft_balance_of(near, &ft_contract_id, alice.account_id()).await?, TOTAL_SUPPLY);
    assert_eq!(ft_balance_of(near, &ft_contract_id, &defi_contract_id).await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_transfer_call_when_called_contract_not_registered_with_ft() -> anyhow::Result<()> {
    let (_node, alice, ft_contract_id, defi_contract_id) = setup().await?;

    let tx_details = alice
        .transaction(&ft_contract_id)
        .add_action(transfer_call(&defi_contract_id, "take-my-money"))
        .await?;

    assert_eq!(
        failures(&tx_details),
        [format!("The account {defi_contract_id} is not registered")]
    );

    // Balances remain unchanged
    let near = alice.near();

    assert_eq!(ft_balance_of(near, &ft_contract_id, alice.account_id()).await?, TOTAL_SUPPLY);
    assert_eq!(ft_balance_of(near, &ft_contract_id, &defi_contract_id).await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_transfer_call_with_burned_amount() -> anyhow::Result<()> {
    let (_node, alice, ft_contract_id, defi_contract_id) = setup().await?;

    register(&alice, &ft_contract_id, &defi_contract_id).await?;

    // In one transaction, Alice sends tokens to the DeFi contract, which returns 10 of them,
    // and closes her token account. Her account is gone by the time the token contract
    // settles the transfer, so the 10 returned tokens are burnt.
    let tx_details = alice
        .transaction(&ft_contract_id)
        .add_action(transfer_call(&defi_contract_id, "10"))
        .add_action(
            FunctionCall::new("storage_unregister")
                .args(json!({ "force": true }))
                .gas(Gas::from_tgas(2))
                .deposit(ONE_YOCTO),
        )
        .await?;

    // The transaction returns what its last action, `storage_unregister`, returns
    let closed: bool = tx_details.json()?;

    assert!(closed);

    let logs: Vec<&str> = tx_details.logs().collect();

    assert!(logs.contains(&"The account of the sender was deleted"));
    assert!(logs.contains(&format!("Account @{} burned 10", alice.account_id()).as_str()));

    // `ft_resolve_transfer` is the token contract's only receipt that returns a token amount.
    // It counts the whole amount as used, since nothing went back to Alice.
    let used: Vec<U128> = tx_details
        .receipts_outcome
        .iter()
        .filter(|receipt| receipt.outcome.executor_id == ft_contract_id)
        .filter_map(|receipt| match &receipt.outcome.status {
            ExecutionStatus::SuccessValue(value) => serde_json::from_slice(value).ok(),
            _ => None,
        })
        .collect();

    assert_eq!(used, [U128(AMOUNT)]);

    // Alice's tokens and the 10 returned ones are burnt, the DeFi contract keeps the rest
    let total_supply: U128 =
        alice.near().view(&ft_contract_id, "ft_total_supply").at(Finality::Optimistic).await?;

    assert_eq!(total_supply.0, AMOUNT - 10);
    assert_eq!(ft_balance_of(alice.near(), &ft_contract_id, &defi_contract_id).await?, AMOUNT - 10);

    Ok(())
}
