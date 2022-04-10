use crate::utils::{init, helper_mint, register_user_for_token};
use near_primitives::views::FinalExecutionStatus;
use near_sdk::json_types::U128;
use near_sdk::{ONE_YOCTO};
use workspaces::AccountId;
use near_contract_standards::multi_token::token::Token;

#[tokio::test]
async fn simulate_mt_transfer_and_call() -> anyhow::Result<()> {
    
    // Setup MT contract, user, and DeFi contract.
    let worker = workspaces::sandbox();
    let (mt, alice, defi) = init(&worker).await?;

    // Mint 2 tokens.
    let token_1: Token = helper_mint(
        &mt,
        &worker,
        alice.id().clone(),
        1000u128,
        "title1".to_string(),
        "desc1".to_string(),
    ).await?;
    let token_2: Token = helper_mint(
        &mt,
        &worker,
        alice.id().clone(),
        20_000u128,
        "title2".to_string(),
        "desc2".to_string(),
    ).await?;

    // Register defi account; alice (the token owner) was already registered during the mint.
    register_user_for_token(&worker, &mt, defi.id(), token_1.token_id.clone()).await?;
    register_user_for_token(&worker, &mt, defi.id(), token_2.token_id.clone()).await?;

    // Transfer some tokens using transfer_and_call to hit DeFi contract with XCC.
    let res = alice
        .call(&worker, mt.id().clone(), "mt_transfer_call")
        .args_json((
            defi.id(),
            token_1.token_id.clone(),
            "100",
            Option::<(AccountId, u64)>::None,
            Option::<String>::None,
            "30", // Number of tokens that the DeFi contract should refund.
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));
    let amounts_kept: Vec<U128> = res.json()?;
    assert_eq!(amounts_kept, vec![U128(70)]);

    let alice_balance: Vec<U128> = mt.call(&worker, "mt_batch_balance_of")
        .args_json((alice.id(), vec![token_1.token_id.clone()], ))?
        .view()
        .await?
        .json()?;
    assert_eq!(alice_balance, vec![U128(930)]);

    let defi_balance: Vec<U128> = mt.call(&worker, "mt_batch_balance_of")
        .args_json((defi.id(), vec![token_1.token_id.clone()], ))?
        .view()
        .await?
        .json()?;
    assert_eq!(defi_balance, vec![U128(70)]);


    // Next, do a batch transfer call, and use special msg 'take-my-money' so DeFi contract refunds nothing.
    let res = alice
        .call(&worker, mt.id().clone(), "mt_batch_transfer_call")
        .args_json((
            defi.id(),
            [token_1.token_id.clone(), token_2.token_id.clone()],
            ["100", "5000"],
            Option::<(AccountId, u64)>::None,
            Option::<String>::None,
            "take-my-money", // DeFi contract will keep all sent tokens.
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));


    // Attempt a transfer where DeFi contract will panic. Token transfer should be reverted in the callback.
    let res = alice
        .call(&worker, mt.id().clone(), "mt_batch_transfer_call")
        .args_json((
            defi.id(),
            [token_1.token_id.clone(), token_2.token_id.clone()],
            ["100", "5000"],
            Option::<(AccountId, u64)>::None,
            Option::<String>::None,
            "not-a-parsable-number",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));
    let amounts_kept_by_receiver: Vec<U128> = res.json()?;
    assert_eq!(amounts_kept_by_receiver, vec![U128(0), U128(0)]);

    // Balance hasn't changed.
    let alice_balance: Vec<U128> = mt.call(&worker, "mt_batch_balance_of")
        .args_json((alice.id(), vec![token_1.token_id.clone(), token_2.token_id.clone()], ))?
        .view()
        .await?
        .json()?;
    assert_eq!(alice_balance, vec![U128(830), U128(15_000)]);


    Ok(())
}
