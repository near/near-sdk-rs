//! Integration tests for MultiToken core functionality (NEP-245)
//! Tests: transfer, batch_transfer, transfer_call, balance, supply

use crate::utils::{
    helper_mint, initialized_contracts, sample_token_metadata, INITIAL_MINT_AMOUNT,
    TOKEN_ID_GOLD, TOKEN_ID_POTION, TOKEN_ID_SWORD,
};
use near_contract_standards::multi_token::Token;
use near_sdk::json_types::U128;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use rstest::rstest;

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

// =============================================================================
// View Methods Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_mt_balance_of(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;

    assert_eq!(balance.0, INITIAL_MINT_AMOUNT);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_batch_balance_of(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let balances: Vec<U128> = contract
        .call("mt_batch_balance_of")
        .args_json((
            contract.id(),
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
        ))
        .view()
        .await?
        .json()?;

    assert_eq!(balances.len(), 2);
    assert_eq!(balances[0].0, INITIAL_MINT_AMOUNT);
    assert_eq!(balances[1].0, INITIAL_MINT_AMOUNT * 10);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_supply(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let supply: Option<U128> = contract
        .call("mt_supply")
        .args_json((TOKEN_ID_SWORD,))
        .view()
        .await?
        .json()?;

    assert_eq!(supply.unwrap().0, INITIAL_MINT_AMOUNT);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_supply_nonexistent_token(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let supply: Option<U128> = contract
        .call("mt_supply")
        .args_json(("nonexistent-token",))
        .view()
        .await?
        .json()?;

    assert!(supply.is_none());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_token(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let tokens: Vec<Option<Token>> = contract
        .call("mt_token")
        .args_json((vec![TOKEN_ID_SWORD],))
        .view()
        .await?
        .json()?;

    assert_eq!(tokens.len(), 1);
    let token = tokens[0].as_ref().unwrap();
    assert_eq!(token.token_id, TOKEN_ID_SWORD);
    assert!(token.metadata.is_some());
    assert_eq!(token.metadata.as_ref().unwrap().title, Some("Silver Sword".to_string()));
    Ok(())
}

// =============================================================================
// Transfer Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_simple_transfer(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;
    let transfer_amount = 100u128;

    // Transfer from contract owner to alice
    let res = contract
        .call("mt_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID_SWORD,
            U128(transfer_amount),
            Option::<(String, u64)>::None,
            Some("test transfer"),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Transfer failed: {:?}", res.failures());

    // Verify balances
    let owner_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    let alice_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((alice.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;

    assert_eq!(owner_balance.0, INITIAL_MINT_AMOUNT - transfer_amount);
    assert_eq!(alice_balance.0, transfer_amount);

    // Verify supply unchanged
    let supply: Option<U128> = contract
        .call("mt_supply")
        .args_json((TOKEN_ID_SWORD,))
        .view()
        .await?
        .json()?;
    assert_eq!(supply.unwrap().0, INITIAL_MINT_AMOUNT);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_to_self_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let res = contract
        .call("mt_transfer")
        .args_json((
            contract.id(), // transfer to self
            TOKEN_ID_SWORD,
            U128(100),
            Option::<(String, u64)>::None,
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_failure());
    assert!(format!("{:?}", res).contains("Cannot transfer to self"));
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_insufficient_balance_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Try to transfer more than balance
    let res = contract
        .call("mt_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID_SWORD,
            U128(INITIAL_MINT_AMOUNT + 1), // more than available
            Option::<(String, u64)>::None,
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_without_deposit_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    let res = contract
        .call("mt_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID_SWORD,
            U128(100),
            Option::<(String, u64)>::None,
            Option::<String>::None,
        ))
        .max_gas()
        // No deposit!
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}

// =============================================================================
// Batch Transfer Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_transfer(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Batch transfer swords and potions
    let res = contract
        .call("mt_batch_transfer")
        .args_json((
            alice.id(),
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            vec![U128(50), U128(200)],
            Option::<Vec<Option<(String, u64)>>>::None,
            Some("batch transfer test"),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Batch transfer failed: {:?}", res.failures());

    // Verify balances
    let balances: Vec<U128> = contract
        .call("mt_batch_balance_of")
        .args_json((alice.id(), vec![TOKEN_ID_SWORD, TOKEN_ID_POTION]))
        .view()
        .await?
        .json()?;

    assert_eq!(balances[0].0, 50);
    assert_eq!(balances[1].0, 200);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_batch_transfer_mismatched_lengths_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Token IDs and amounts have different lengths
    let res = contract
        .call("mt_batch_transfer")
        .args_json((
            alice.id(),
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            vec![U128(50)], // Only one amount!
            Option::<Vec<Option<(String, u64)>>>::None,
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}

// =============================================================================
// Transfer Call Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_transfer_call_keep_tokens(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, receiver_contract) = initialized_contracts.await?;
    let transfer_amount = 100u128;

    let res = contract
        .call("mt_transfer_call")
        .args_json((
            receiver_contract.id(),
            TOKEN_ID_SWORD,
            U128(transfer_amount),
            Option::<(String, u64)>::None,
            Some("transfer call test"),
            "keep-all-now",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Transfer call failed: {:?}", res.failures());

    // Receiver should have the tokens
    let receiver_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((receiver_contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(receiver_balance.0, transfer_amount);

    // Sender should have reduced balance
    let sender_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(sender_balance.0, INITIAL_MINT_AMOUNT - transfer_amount);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_call_return_all_tokens(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, receiver_contract) = initialized_contracts.await?;
    let transfer_amount = 100u128;

    let res = contract
        .call("mt_transfer_call")
        .args_json((
            receiver_contract.id(),
            TOKEN_ID_SWORD,
            U128(transfer_amount),
            Option::<(String, u64)>::None,
            Some("transfer call test"),
            "return-all-now",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Transfer call failed: {:?}", res.failures());

    // Sender should have all tokens back
    let sender_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(sender_balance.0, INITIAL_MINT_AMOUNT);

    // Receiver should have no tokens
    let receiver_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((receiver_contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(receiver_balance.0, 0);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_call_return_half_tokens(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, receiver_contract) = initialized_contracts.await?;
    let transfer_amount = 100u128;

    let res = contract
        .call("mt_transfer_call")
        .args_json((
            receiver_contract.id(),
            TOKEN_ID_SWORD,
            U128(transfer_amount),
            Option::<(String, u64)>::None,
            Some("transfer call test"),
            "return-half-now",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Transfer call failed: {:?}", res.failures());

    // Sender should have half refunded
    let sender_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(sender_balance.0, INITIAL_MINT_AMOUNT - transfer_amount / 2);

    // Receiver should have half
    let receiver_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((receiver_contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(receiver_balance.0, transfer_amount / 2);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_transfer_call_receiver_panics_refunds_all(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, receiver_contract) = initialized_contracts.await?;
    let transfer_amount = 100u128;

    let res = contract
        .call("mt_transfer_call")
        .args_json((
            receiver_contract.id(),
            TOKEN_ID_SWORD,
            U128(transfer_amount),
            Option::<(String, u64)>::None,
            Option::<String>::None,
            "invalid-message-causes-panic",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    
    // The outer call succeeds (the panic is handled by mt_resolve_transfer)
    assert!(res.is_success(), "Transfer call failed: {:?}", res.failures());

    // Sender should have all tokens back due to receiver panic
    let sender_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(sender_balance.0, INITIAL_MINT_AMOUNT);

    Ok(())
}

// =============================================================================
// Batch Transfer Call Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_batch_transfer_call(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, receiver_contract) = initialized_contracts.await?;

    let res = contract
        .call("mt_batch_transfer_call")
        .args_json((
            receiver_contract.id(),
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            vec![U128(50), U128(100)],
            Option::<Vec<Option<(String, u64)>>>::None,
            Some("batch transfer call"),
            "keep-all-now",
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Batch transfer call failed: {:?}", res.failures());

    // Verify receiver balances
    let balances: Vec<U128> = contract
        .call("mt_batch_balance_of")
        .args_json((receiver_contract.id(), vec![TOKEN_ID_SWORD, TOKEN_ID_POTION]))
        .view()
        .await?
        .json()?;

    assert_eq!(balances[0].0, 50);
    assert_eq!(balances[1].0, 100);
    Ok(())
}

// =============================================================================
// Mint and Burn Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_mint_new_token(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    let metadata = sample_token_metadata("Gold Coin", "Currency");
    helper_mint(
        &contract,
        contract.as_account(),
        TOKEN_ID_GOLD.to_string(),
        alice.id(),
        5000,
        Some(metadata),
    )
    .await?;

    // Verify balance
    let balance: U128 = contract
        .call("mt_balance_of")
        .args_json((alice.id(), TOKEN_ID_GOLD))
        .view()
        .await?
        .json()?;
    assert_eq!(balance.0, 5000);

    // Verify supply
    let supply: Option<U128> = contract
        .call("mt_supply")
        .args_json((TOKEN_ID_GOLD,))
        .view()
        .await?
        .json()?;
    assert_eq!(supply.unwrap().0, 5000);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mint_additional_to_existing_token(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Mint more of existing token to alice
    helper_mint(
        &contract,
        contract.as_account(),
        TOKEN_ID_SWORD.to_string(),
        alice.id(),
        500,
        None,
    )
    .await?;

    // Total supply should increase
    let supply: Option<U128> = contract
        .call("mt_supply")
        .args_json((TOKEN_ID_SWORD,))
        .view()
        .await?
        .json()?;
    assert_eq!(supply.unwrap().0, INITIAL_MINT_AMOUNT + 500);

    // Alice should have her minted amount
    let alice_balance: U128 = contract
        .call("mt_balance_of")
        .args_json((alice.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(alice_balance.0, 500);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_burn(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;
    let burn_amount = 300u128;

    let res = contract
        .call("mt_burn")
        .args_json((TOKEN_ID_SWORD, U128(burn_amount), Some("burning tokens")))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success(), "Burn failed: {:?}", res.failures());

    // Verify balance reduced
    let balance: U128 = contract
        .call("mt_balance_of")
        .args_json((contract.id(), TOKEN_ID_SWORD))
        .view()
        .await?
        .json()?;
    assert_eq!(balance.0, INITIAL_MINT_AMOUNT - burn_amount);

    // Verify supply reduced
    let supply: Option<U128> = contract
        .call("mt_supply")
        .args_json((TOKEN_ID_SWORD,))
        .view()
        .await?
        .json()?;
    assert_eq!(supply.unwrap().0, INITIAL_MINT_AMOUNT - burn_amount);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_burn_more_than_balance_fails(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    let res = contract
        .call("mt_burn")
        .args_json((TOKEN_ID_SWORD, U128(INITIAL_MINT_AMOUNT + 1), Option::<String>::None))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(res.is_failure());
    Ok(())
}

// =============================================================================
// Events Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_transfer_emits_event(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    let res = contract
        .call("mt_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID_SWORD,
            U128(100),
            Option::<(String, u64)>::None,
            Some("event test"),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Should have logged an mt_transfer event
    let logs = res.logs();
    assert!(!logs.is_empty(), "Expected event log");
    assert!(
        logs.iter().any(|log| log.contains("mt_transfer")),
        "Expected mt_transfer event in logs: {:?}",
        logs
    );

    Ok(())
}
