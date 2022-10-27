#[cfg(test)]
mod tests {
    use near_sdk::json_types::U128;
    use near_contract_standards::multi_token::token::Token;
    use near_sdk::ONE_YOCTO;
    use near_units::parse_near;
    use workspaces::AccountId;

    use crate::utils::{get_storage_balance_bounds, helper_mint, init, register_user_for_token};

    #[tokio::test]
    async fn simulate_mt_transfer_with_approval() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, bob, _) = init(&worker).await?;

        let charlie = mt
            .as_account()
            .create_subaccount("charlie")
            .initial_balance(parse_near!("10 N"))
            .transact()
            .await?
            .into_result()?;

        let token: Token = helper_mint(&mt, alice.id().clone(), 1000u128, "title1".to_string(), "desc1".to_string()).await?;

        // Grant bob an approval to take 50 of alice's tokens.
        let _ = alice.call(mt.id(), "mt_approve")
            .args_json((
                [token.token_id.clone()],
                [U128(50)],
                bob.id(),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(490000000000000000000)
            .transact()
            .await?;

        register_user_for_token(
            &mt,
            charlie.id(),
            get_storage_balance_bounds(&mt).await?.min.into(),
        ).await?;

        // Bob tries to transfer 50 tokens to charlie
        let res = bob
            .call(mt.id(), "mt_transfer")
            .args_json((
                charlie.id(),
                token.token_id.clone(),
                "50",
                Option::<(AccountId, u64)>::Some((alice.id().clone(), 0)),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;

        assert!(res.is_success());

        // Bob tries to transfer 50 tokens to charlie, but fails because of insufficient approval.
        let res = bob
            .call(mt.id(), "mt_transfer")
            .args_json((
                charlie.id(),
                token.token_id.clone(),
                "50",
                Option::<(AccountId, u64)>::Some((alice.id().clone(), 0)),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;

        assert!(res.is_failure());

        Ok(())
    }


    #[tokio::test]
    async fn simulate_mt_transfer_wrong_approval() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, bob, _) = init(&worker).await?;

        let charlie = mt
            .as_account()
            .create_subaccount("charlie")
            .initial_balance(parse_near!("10 N"))
            .transact()
            .await?
            .into_result()?;

        let token: Token = helper_mint(&mt, alice.id().clone(), 1000u128, "title1".to_string(), "desc1".to_string()).await?;

        // Grant bob an approval to take 50 of alice's tokens.
        let _ = alice.call(mt.id(), "mt_approve")
            .args_json((
                [token.token_id.clone()],
                [U128(50)],
                bob.id(),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(490000000000000000000)
            .transact()
            .await?;

        // register charlie
        let _ = charlie.call(mt.id(), "register")
            .args_json((
                token.token_id.clone(),
                charlie.id(),
            ))
            .max_gas()
            .transact()
            .await?;

        // charlie tries to transfer 50 tokens to himself, but fails because of wrong approval.
        let res = charlie
            .call(mt.id(), "mt_transfer")
            .args_json((
                charlie.id(),
                token.token_id.clone(),
                "50",
                Option::<(AccountId, u64)>::Some((alice.id().clone(), 0)),
                Option::<String>::None,
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;

        println!("res = {:?}", res);
        assert!(res.is_failure());

        Ok(())
    }


    #[tokio::test]
    async fn simulate_mt_transfer_and_call() -> anyhow::Result<()> {
        // Setup MT contract, user, and DeFi contract.
        let worker = workspaces::sandbox().await?;
        let (mt, alice, _, defi) = init(&worker).await?;

        // Mint 2 tokens.
        let token_1: Token = helper_mint(
            &mt,
            alice.id().clone(),
            1000u128,
            "title1".to_string(),
            "desc1".to_string(),
        ).await?;
        let token_2: Token = helper_mint(
            &mt,
            alice.id().clone(),
            20_000u128,
            "title2".to_string(),
            "desc2".to_string(),
        ).await?;


        // Register defi account; alice (the token owner) was already registered during the mint.
        register_user_for_token(
            &mt,
            defi.id(),
            get_storage_balance_bounds(&mt).await?.min.into(),
        ).await?;

        // Transfer some tokens using transfer_and_call to hit DeFi contract with XCC.
        let res = alice
            .call(mt.id(), "mt_transfer_call")
            .args_json((
                defi.id(),
                token_1.token_id.clone(),
                "100",
                Option::<(AccountId, u64)>::None,
                Option::<String>::None,
                "30", // Number of tokens that the DeFi contract should refund.
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;
        assert!(res.is_success());
        let amounts_kept: Vec<U128> = res.json()?;
        assert_eq!(amounts_kept, vec![U128(70)]);

        let alice_balance: Vec<U128> = mt.call("mt_batch_balance_of")
            .args_json((alice.id(), vec![token_1.token_id.clone()], ))
            .view()
            .await?
            .json()?;
        assert_eq!(alice_balance, vec![U128(930)]);

        let defi_balance: Vec<U128> = mt.call("mt_batch_balance_of")
            .args_json((defi.id(), vec![token_1.token_id.clone()], ))
            .view()
            .await?
            .json()?;
        assert_eq!(defi_balance, vec![U128(70)]);


        // Next, do a batch transfer call, and use special msg 'take-my-money' so DeFi contract refunds nothing.
        let res = alice
            .call(mt.id(), "mt_batch_transfer_call")
            .args_json((
                defi.id(),
                [token_1.token_id.clone(), token_2.token_id.clone()],
                ["100", "5000"],
                Option::<(AccountId, u64)>::None,
                Option::<String>::None,
                "take-my-money", // DeFi contract will keep all sent tokens.
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;
        assert!(res.is_success());


        // Attempt a transfer where DeFi contract will panic. Token transfer should be reverted in the callback.
        let res = alice
            .call(mt.id(), "mt_batch_transfer_call")
            .args_json((
                defi.id(),
                [token_1.token_id.clone(), token_2.token_id.clone()],
                ["100", "5000"],
                Option::<(AccountId, u64)>::None,
                Option::<String>::None,
                "not-a-parsable-number",
            ))
            .max_gas()
            .deposit(ONE_YOCTO)
            .transact()
            .await?;
        assert!(res.is_success());
        let amounts_kept_by_receiver: Vec<U128> = res.json()?;
        assert_eq!(amounts_kept_by_receiver, vec![U128(0), U128(0)]);

        // Balance hasn't changed.
        let alice_balance: Vec<U128> = mt.call("mt_batch_balance_of")
            .args_json((alice.id(), vec![token_1.token_id.clone(), token_2.token_id.clone()], ))
            .view()
            .await?
            .json()?;
        assert_eq!(alice_balance, vec![U128(830), U128(15_000)]);

        Ok(())
    }
}
