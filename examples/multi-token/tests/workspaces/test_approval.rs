use crate::utils::{helper_mint, init, init_approval_receiver_contract};
use near_contract_standards::multi_token::token::Token;

#[tokio::test]
async fn simulate_mt_approval_with_receiver() -> anyhow::Result<()> {
  let worker = workspaces::sandbox();
  let (mt, alice, _) = init(&worker).await?;
  let approval_receiver = init_approval_receiver_contract(&worker).await?;

  let token: Token = helper_mint(&mt, &worker, alice.id().clone(), 1000u128, "title1".to_string(), "desc1".to_string()).await?;

  // Grant approval_receiver contract an approval to take 50 of alice's tokens.
  let res = alice.call(&worker, mt.id().clone(), "mt_approve")
    .args_json((
      [token.token_id.clone()],
      [50u64],
      approval_receiver.id(),
      Option::<String>::Some("some-msg".to_string()),
    ))?
    .gas(300_000_000_000_000)
    .deposit(450000000000000000000)
    .transact()
    .await?;
  assert_eq!(res.json::<String>()?, "yeeeeeeeeeeeeeeee".to_string());

  Ok(())
}
