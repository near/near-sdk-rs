//! Integration tests for MultiToken enumeration functionality (NEP-245)
//! Tests: mt_tokens, mt_tokens_for_owner

use crate::utils::{
    helper_mint, initialized_contracts, sample_token_metadata,
    TOKEN_ID_GOLD, TOKEN_ID_POTION, TOKEN_ID_SWORD,
};
use near_contract_standards::multi_token::Token;
use near_sdk::json_types::U128;
use near_workspaces::types::NearToken;
use near_workspaces::{Account, Contract};
use rstest::rstest;

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

// =============================================================================
// Enumeration Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_mt_tokens(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    // Get all tokens
    let tokens: Vec<Token> = contract
        .call("mt_tokens")
        .args_json((Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;

    // Should have sword and potion from initial setup
    assert_eq!(tokens.len(), 2);
    
    let token_ids: Vec<&str> = tokens.iter().map(|t| t.token_id.as_str()).collect();
    assert!(token_ids.contains(&TOKEN_ID_SWORD));
    assert!(token_ids.contains(&TOKEN_ID_POTION));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_tokens_pagination(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, _, _, _) = initialized_contracts.await?;

    // Mint a third token
    helper_mint(
        &contract,
        contract.as_account(),
        TOKEN_ID_GOLD.to_string(),
        contract.id(),
        1000,
        Some(sample_token_metadata("Gold", "Currency")),
    )
    .await?;

    // Get first token only (limit=1)
    let tokens: Vec<Token> = contract
        .call("mt_tokens")
        .args_json((Option::<U128>::None, Some(1u32)))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);

    // Get tokens starting from index 1
    let tokens: Vec<Token> = contract
        .call("mt_tokens")
        .args_json((Some(U128(1)), Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 2); // Should have 2 more tokens

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_tokens_for_owner(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Transfer some tokens to alice
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
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Get alice's tokens
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_id, TOKEN_ID_SWORD);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_tokens_for_owner_multiple_tokens(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Transfer sword and potion to alice
    let res = contract
        .call("mt_batch_transfer")
        .args_json((
            alice.id(),
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            vec![U128(50), U128(100)],
            Option::<Vec<Option<(String, u64)>>>::None,
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Get alice's tokens
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;

    assert_eq!(tokens.len(), 2);
    let token_ids: Vec<&str> = tokens.iter().map(|t| t.token_id.as_str()).collect();
    assert!(token_ids.contains(&TOKEN_ID_SWORD));
    assert!(token_ids.contains(&TOKEN_ID_POTION));

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_tokens_for_owner_empty(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Alice has no tokens initially
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;

    assert!(tokens.is_empty());

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_token_removed_from_owner_after_full_transfer(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, bob, _) = initialized_contracts.await?;

    // Mint tokens directly to alice
    helper_mint(
        &contract,
        contract.as_account(),
        TOKEN_ID_GOLD.to_string(),
        alice.id(),
        100,
        None,
    )
    .await?;

    // Verify alice has the token
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);

    // Alice transfers ALL her gold to bob
    let res = alice
        .call(contract.id(), "mt_transfer")
        .args_json((
            bob.id(),
            TOKEN_ID_GOLD,
            U128(100),
            Option::<(String, u64)>::None,
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Alice should no longer have the token in her list
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert!(tokens.is_empty());

    // Bob should have it
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((bob.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_id, TOKEN_ID_GOLD);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_token_removed_after_burn_all(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Mint tokens to alice
    helper_mint(
        &contract,
        contract.as_account(),
        TOKEN_ID_GOLD.to_string(),
        alice.id(),
        100,
        None,
    )
    .await?;

    // Verify alice has the token
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);

    // Alice burns all her gold
    let res = alice
        .call(contract.id(), "mt_burn")
        .args_json((TOKEN_ID_GOLD, U128(100), Option::<String>::None))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Alice should no longer have the token in her list
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert!(tokens.is_empty());

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_mt_tokens_for_owner_pagination(
    #[future] initialized_contracts: anyhow::Result<(Contract, Account, Account, Contract)>,
) -> anyhow::Result<()> {
    let (contract, alice, _, _) = initialized_contracts.await?;

    // Give alice multiple token types
    let res = contract
        .call("mt_batch_transfer")
        .args_json((
            alice.id(),
            vec![TOKEN_ID_SWORD, TOKEN_ID_POTION],
            vec![U128(50), U128(100)],
            Option::<Vec<Option<(String, u64)>>>::None,
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // Get first token only (limit=1)
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Some(1u32)))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);

    // Get all tokens
    let tokens: Vec<Token> = contract
        .call("mt_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u32>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 2);

    Ok(())
}
