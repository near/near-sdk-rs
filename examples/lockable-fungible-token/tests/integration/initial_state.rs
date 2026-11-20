use near_kit::json_types::U128;
use serde_json::json;

use crate::shared::{TOTAL_SUPPLY, allowance, locked_balance, setup, unlocked_balance};

#[tokio::test]
async fn test_owner_initial_state() -> anyhow::Result<()> {
    let (_node, alice, _, contract_id) = setup().await?;
    let near = alice.near();
    let alice_id = alice.account_id();

    let total_supply: U128 = near.view(&contract_id, "get_total_supply").await?;

    assert_eq!(total_supply.0, TOTAL_SUPPLY);

    // Alice owns the whole supply, none of it locked
    let total_balance: U128 =
        near.view(&contract_id, "get_total_balance").args(json!({ "owner_id": alice_id })).await?;

    assert_eq!(total_balance.0, TOTAL_SUPPLY);
    assert_eq!(unlocked_balance(near, &contract_id, alice_id).await?, TOTAL_SUPPLY);
    assert_eq!(allowance(near, &contract_id, alice_id, alice_id).await?, 0);
    assert_eq!(locked_balance(near, &contract_id, alice_id, alice_id).await?, 0);

    Ok(())
}
