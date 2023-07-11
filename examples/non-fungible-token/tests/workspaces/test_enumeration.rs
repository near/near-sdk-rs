use crate::utils::{helper_mint, init};
use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::U128;
use workspaces::Contract;

async fn mint_more(nft_contract: &Contract) -> anyhow::Result<()> {
    helper_mint(
        nft_contract,
        "1".to_string(),
        "Black as the Night".to_string(),
        "In charcoal".to_string(),
    )
    .await?;
    helper_mint(
        nft_contract,
        "2".to_string(),
        "Hamakua".to_string(),
        "Vintage recording".to_string(),
    )
    .await?;
    helper_mint(
        nft_contract,
        "3".to_string(),
        "Aloha ke akua".to_string(),
        "Original with piano".to_string(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn simulate_enum_total_supply() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, _, _) = init(&worker).await?;
    mint_more(&nft_contract).await?;

    let total_supply: U128 = nft_contract.call("nft_total_supply").view().await?.json()?;
    assert_eq!(total_supply, U128::from(4));

    Ok(())
}

#[tokio::test]
async fn simulate_enum_nft_tokens() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, _, _, _) = init(&worker).await?;
    mint_more(&nft_contract).await?;

    // No optional args should return all
    let mut tokens: Vec<Token> = nft_contract
        .call("nft_tokens")
        .args_json((Option::<U128>::None, Option::<u64>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 4);
    // Start at "1", with no limit arg
    tokens = nft_contract
        .call("nft_tokens")
        .args_json((Some(U128::from(1)), Option::<u64>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens.get(0).unwrap().token_id, "1".to_string());
    assert_eq!(tokens.get(1).unwrap().token_id, "2".to_string());
    assert_eq!(tokens.get(2).unwrap().token_id, "3".to_string());

    // Start at "2", with limit 1
    tokens = nft_contract
        .call("nft_tokens")
        .args_json((Some(U128::from(2)), Some(1u64)))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens.get(0).unwrap().token_id, "2".to_string());

    // Don't specify from_index, but limit 2
    tokens = nft_contract
        .call("nft_tokens")
        .args_json((Option::<U128>::None, Some(2u64)))
        .view()
        .await?
        .json()?;
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens.get(0).unwrap().token_id, "0".to_string());
    assert_eq!(tokens.get(1).unwrap().token_id, "1".to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_enum_nft_supply_for_owner() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, alice, _, _) = init(&worker).await?;

    // Get number from account with no NFTs
    let owner_num_tokens: U128 = nft_contract
        .call("nft_supply_for_owner")
        .args_json((alice.id(),))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_num_tokens, U128::from(0));

    let owner_num_tokens: U128 = nft_contract
        .call("nft_supply_for_owner")
        .args_json((nft_contract.id(),))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_num_tokens, U128::from(1));

    mint_more(&nft_contract).await?;

    let owner_num_tokens: U128 = nft_contract
        .call("nft_supply_for_owner")
        .args_json((nft_contract.id(),))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_num_tokens, U128::from(4));

    Ok(())
}

#[tokio::test]
async fn simulate_enum_nft_tokens_for_owner() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (nft_contract, alice, _, _) = init(&worker).await?;
    mint_more(&nft_contract).await?;

    // Get tokens from account with no NFTs
    let owner_tokens: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json((alice.id(), Option::<U128>::None, Option::<u64>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_tokens.len(), 0);

    // Get tokens with no optional args
    let owner_tokens: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json((nft_contract.id(), Option::<U128>::None, Option::<u64>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_tokens.len(), 4);

    // With from_index and no limit
    let owner_tokens: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json((nft_contract.id(), Some(U128::from(2)), Option::<u64>::None))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_tokens.len(), 2);
    assert_eq!(owner_tokens.get(0).unwrap().token_id, "2".to_string());
    assert_eq!(owner_tokens.get(1).unwrap().token_id, "3".to_string());

    // With from_index and limit 1
    let owner_tokens: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json((nft_contract.id(), Some(U128::from(1)), Some(1u64)))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_tokens.len(), 1);
    assert_eq!(owner_tokens.get(0).unwrap().token_id, "1".to_string());

    // No from_index but limit 3
    let owner_tokens: Vec<Token> = nft_contract
        .call("nft_tokens_for_owner")
        .args_json((nft_contract.id(), Option::<U128>::None, Some(3u64)))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_tokens.len(), 3);
    assert_eq!(owner_tokens.get(0).unwrap().token_id, "0".to_string());
    assert_eq!(owner_tokens.get(1).unwrap().token_id, "1".to_string());
    assert_eq!(owner_tokens.get(2).unwrap().token_id, "2".to_string());

    Ok(())
}
