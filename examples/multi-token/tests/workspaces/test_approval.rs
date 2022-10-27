use near_sdk::json_types::U128;
use approval_receiver::ON_MT_TOKEN_APPROVE_MSG;
use near_contract_standards::multi_token::token::Token;

use crate::utils::{helper_mint, init, init_approval_receiver_contract};

#[tokio::test]
async fn simulate_mt_approval_with_receiver() -> anyhow::Result<()> {
    let worker = workspaces::sandbox().await?;
    let (mt, alice, _, _) = init(&worker).await?;
    let approval_receiver = init_approval_receiver_contract(&worker).await?;

    let token: Token = helper_mint(&mt, alice.id().clone(), 1000u128, "title1".to_string(), "desc1".to_string()).await?;

    // Grant approval_receiver contract an approval to take 50 of alice's tokens.
    let res = alice.call(mt.id(), "mt_approve")
        .args_json((
            [token.token_id.clone()],
            [U128(50)],
            approval_receiver.id(),
            Option::<String>::Some("some-msg".to_string()),
        ))
        .max_gas()
        .deposit(450000000000000000000)
        .transact()
        .await?
        .json::<String>()?;
    assert_eq!(res, ON_MT_TOKEN_APPROVE_MSG.to_string());

    Ok(())
}
