#[cfg(test)]
mod tests {
    use approval_receiver::ON_MT_TOKEN_APPROVE_MSG;
    use near_contract_standards::multi_token::token::Token;
    use near_sdk::json_types::U128;
    use near_sdk::ONE_YOCTO;
    use workspaces::AccountId;

    use crate::utils::{
        get_storage_balance_bounds, helper_mint, init, init_approval_receiver_contract,
        register_user_for_token,
    };

    #[tokio::test]
    async fn simulate_mt_approval_with_receiver() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, _, _) = init(&worker).await?;
        let approval_receiver = init_approval_receiver_contract(&worker).await?;

        let token: Token = helper_mint(
            &mt,
            alice.id().clone(),
            1000u128,
            "title1".to_string(),
            "desc1".to_string(),
        )
        .await?;

        // Grant approval_receiver contract an approval to take 50 of alice's tokens.
        let res = alice
            .call(mt.id(), "mt_approve")
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

    #[tokio::test]
    async fn test_approval_after_failed_mt_transfer_call() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, _, defi) = init(&worker).await?;

        let token: Token = helper_mint(
            &mt,
            alice.id().clone(),
            1000u128,
            "title1".to_string(),
            "desc1".to_string(),
        )
        .await?;

        register_user_for_token(&mt, defi.id(), get_storage_balance_bounds(&mt).await?.min.into())
            .await?;

        register_user_for_token(&mt, alice.id(), get_storage_balance_bounds(&mt).await?.min.into())
            .await?;

        let approve_amount = 50;
        // approve defi to take 50 of alice's tokens.
        let res = alice
            .call(mt.id(), "mt_approve")
            .args_json((
                [token.token_id.clone()],
                [U128(approve_amount)],
                defi.id(),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(450000000000000000000)
            .transact()
            .await?;
        assert!(res.is_success());

        let is_approved: bool = mt
            .call("mt_is_approved")
            .args_json((
                alice.id(),
                [token.token_id.clone()],
                defi.id(),
                [U128(approve_amount)],
                Option::<Vec<u64>>::None,
            ))
            .view()
            .await?
            .json()?;

        assert!(is_approved);

        let _ = defi
            .as_account()
            .call(mt.id(), "mt_transfer_call")
            .args_json((
                defi.id(),
                token.token_id.clone(),
                "50",
                Option::<(AccountId, u64)>::Some((alice.id().clone(), 0)),
                Option::<String>::None,
                "fail",
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;

        let is_approved: bool = mt
            .call("mt_is_approved")
            .args_json((
                alice.id(),
                [token.token_id.clone()],
                defi.id(),
                [U128(approve_amount)],
                Option::<Vec<u64>>::None,
            ))
            .view()
            .await?
            .json()?;

        assert!(is_approved);

        Ok(())
    }

    /**
    In this test we simulate the following scenario:
      1. Alice grants approval to defi contract to take 500 of token_1.
      2. Bob grants approval to defi contract to take 1000 of token_2.
      3. Defi contract transfers 500 of token_1 and 900 of token_2 and fails.
      4. We check that Alice's and Bob's approvals is still valid for 500 and 1000 tokens.
     */
    #[tokio::test]
    async fn test_approval_after_failed_mt_batch_transfer_call() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, bob, defi) = init(&worker).await?;

        let token_1: Token = helper_mint(
            &mt,
            alice.id().clone(),
            500u128,
            "token_1".to_string(),
            "desc".to_string(),
        )
        .await?;
        let token_2: Token =
            helper_mint(&mt, bob.id().clone(), 1000u128, "token_2".to_string(), "desc".to_string())
                .await?;

        register_user_for_token(&mt, defi.id(), get_storage_balance_bounds(&mt).await?.min.0 * 2)
            .await?;

        register_user_for_token(&mt, alice.id(), get_storage_balance_bounds(&mt).await?.min.into())
            .await?;

        register_user_for_token(&mt, bob.id(), get_storage_balance_bounds(&mt).await?.min.into())
            .await?;

        // approve defi to take 500 of alice's tokens.
        let alice_approve_amount = 500;
        let res = alice
            .call(mt.id(), "mt_approve")
            .args_json((
                [token_1.token_id.clone()],
                [U128(alice_approve_amount)],
                defi.id(),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(450000000000000000000)
            .transact()
            .await?;
        assert!(res.is_success());

        // approve defi to take 1000 of bob's tokens.
        let bob_approve_amount = 1000;
        let res = bob
            .call(mt.id(), "mt_approve")
            .args_json((
                [token_2.token_id.clone()],
                [U128(bob_approve_amount)],
                defi.id(),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(450000000000000000000)
            .transact()
            .await?;
        assert!(res.is_success());

        let is_approved: bool = mt
            .call("mt_is_approved")
            .args_json((
                alice.id(),
                [token_1.token_id.clone()],
                defi.id(),
                [U128(alice_approve_amount)],
                Option::<Vec<u64>>::None,
            ))
            .view()
            .await?
            .json()?;
        assert!(is_approved);

        let is_approved: bool = mt
            .call("mt_is_approved")
            .args_json((
                bob.id(),
                [token_2.token_id.clone()],
                defi.id(),
                [U128(bob_approve_amount)],
                Option::<Vec<u64>>::None,
            ))
            .view()
            .await?
            .json()?;
        assert!(is_approved);

        let _ = defi
            .as_account()
            .call(mt.id(), "mt_batch_transfer_call")
            .args_json((
                defi.id(),
                [token_1.token_id.clone(), token_2.token_id.clone()],
                ["500", "900"], // from bob we will take 900, when he has 1000
                Option::<(AccountId, u64)>::None,
                Option::<String>::None,
                "fail", // DeFi contract will rollback the transfer
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;

        let is_approved: bool = mt
            .call("mt_is_approved")
            .args_json((
                alice.id(),
                [token_1.token_id.clone()],
                defi.id(),
                [U128(alice_approve_amount)],
                Option::<Vec<u64>>::None,
            ))
            .view()
            .await?
            .json()?;
        assert!(is_approved);

        let is_approved: bool = mt
            .call("mt_is_approved")
            .args_json((
                bob.id(),
                [token_2.token_id.clone()],
                defi.id(),
                [U128(bob_approve_amount)],
                Option::<Vec<u64>>::None,
            ))
            .view()
            .await?
            .json()?;
        assert!(is_approved);

        Ok(())
    }
}
