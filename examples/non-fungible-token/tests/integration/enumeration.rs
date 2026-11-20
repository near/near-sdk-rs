use near_kit::json_types::U128;
use near_kit::standards::NftToken;
use near_kit::{AccountId, Near, SigningNear};
use serde_json::{Value, json};

use crate::shared::{mint, setup};

/// Mints three more tokens to Alice, who then owns the tokens "0" to "3".
async fn mint_more(alice: &SigningNear, nft_contract_id: &AccountId) -> anyhow::Result<()> {
    mint(alice, nft_contract_id, "1", "Black as the Night", "In charcoal").await?;
    mint(alice, nft_contract_id, "2", "Hamakua", "Vintage recording").await?;
    mint(alice, nft_contract_id, "3", "Aloha ke akua", "Original with piano").await?;

    Ok(())
}

/// The ids of the tokens that the view `method` lists for `args`.
async fn token_ids(
    near: &Near,
    nft_contract_id: &AccountId,
    method: &str,
    args: &Value,
) -> anyhow::Result<Vec<String>> {
    let tokens: Vec<NftToken> = near.view(nft_contract_id, method).args(args).await?;

    Ok(tokens.into_iter().map(|token| token.token_id).collect())
}

/// How many tokens `account_id` owns.
async fn nft_supply_for_owner(
    near: &Near,
    nft_contract_id: &AccountId,
    account_id: &AccountId,
) -> anyhow::Result<u128> {
    let supply: U128 = near
        .view(nft_contract_id, "nft_supply_for_owner")
        .args(json!({ "account_id": account_id }))
        .await?;

    Ok(supply.0)
}

#[tokio::test]
async fn test_enum_total_supply() -> anyhow::Result<()> {
    let (_node, alice, _, nft_contract_id) = setup().await?;

    mint_more(&alice, &nft_contract_id).await?;

    let total_supply: U128 = alice.near().view(&nft_contract_id, "nft_total_supply").await?;

    assert_eq!(total_supply.0, 4);

    Ok(())
}

#[tokio::test]
async fn test_enum_nft_tokens() -> anyhow::Result<()> {
    let (_node, alice, _, nft_contract_id) = setup().await?;

    mint_more(&alice, &nft_contract_id).await?;

    // Each case is `(from_index, limit)` and the tokens listed
    let cases = [
        ((None, None), vec!["0", "1", "2", "3"]),
        ((Some(1), None), vec!["1", "2", "3"]),
        ((Some(2), Some(1)), vec!["2"]),
        ((None, Some(2)), vec!["0", "1"]),
    ];

    for ((from_index, limit), listed) in cases {
        let args = json!({
            "from_index": from_index.map(U128),
            "limit": limit,
        });

        assert_eq!(
            token_ids(alice.near(), &nft_contract_id, "nft_tokens", &args).await?,
            listed,
            "nft_tokens({args})"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_enum_nft_supply_for_owner() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;
    let near = alice.near();

    // Bob owns no tokens, Alice owns the one minted in setup
    assert_eq!(nft_supply_for_owner(near, &nft_contract_id, bob.account_id()).await?, 0);
    assert_eq!(nft_supply_for_owner(near, &nft_contract_id, alice.account_id()).await?, 1);

    mint_more(&alice, &nft_contract_id).await?;

    assert_eq!(nft_supply_for_owner(near, &nft_contract_id, alice.account_id()).await?, 4);

    Ok(())
}

#[tokio::test]
async fn test_enum_nft_tokens_for_owner() -> anyhow::Result<()> {
    let (_node, alice, bob, nft_contract_id) = setup().await?;

    mint_more(&alice, &nft_contract_id).await?;

    // Each case is `(account_id, from_index, limit)` and the tokens listed
    let cases = [
        ((bob.account_id(), None, None), vec![]),
        ((alice.account_id(), None, None), vec!["0", "1", "2", "3"]),
        ((alice.account_id(), Some(2), None), vec!["2", "3"]),
        ((alice.account_id(), Some(1), Some(1)), vec!["1"]),
        ((alice.account_id(), None, Some(3)), vec!["0", "1", "2"]),
    ];

    for ((account_id, from_index, limit), listed) in cases {
        let args = json!({
            "account_id": account_id,
            "from_index": from_index.map(U128),
            "limit": limit,
        });

        assert_eq!(
            token_ids(alice.near(), &nft_contract_id, "nft_tokens_for_owner", &args).await?,
            listed,
            "nft_tokens_for_owner({args})"
        );
    }

    Ok(())
}
