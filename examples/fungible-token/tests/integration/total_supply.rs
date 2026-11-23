use near_kit::json_types::U128;
use near_kit::rpc::Finality;

use crate::shared::{TOTAL_SUPPLY, setup};

#[tokio::test]
async fn test_total_supply() -> anyhow::Result<()> {
    let (_node, alice, _, ft_contract_id) = setup().await?;

    let total_supply: U128 =
        alice.near().view(&ft_contract_id, "ft_total_supply").at(Finality::Optimistic).await?;

    assert_eq!(total_supply.0, TOTAL_SUPPLY);

    Ok(())
}
