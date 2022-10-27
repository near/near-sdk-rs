#[cfg(test)]
mod tests {
    use crate::utils::{helper_mint, init};
    use near_contract_standards::multi_token::token::Token;
    use near_sdk::json_types::U128;

    #[tokio::test]
    async fn simulate_enum_all_tokens() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, _, _) = init(&worker).await?;

        // Mint 3 tokens
        let token_1: Token = helper_mint(&mt, alice.id().clone(), 1000u128, "title1".to_string(), "desc1".to_string()).await?;
        let token_2: Token = helper_mint(&mt, alice.id().clone(), 20_000u128, "title2".to_string(), "desc2".to_string()).await?;
        let token_3: Token = helper_mint(&mt, alice.id().clone(), 5u128, "title3".to_string(), "desc3".to_string()).await?;

        // Get all tokens
        let res: Vec<Token> = mt.call("mt_tokens")
            .args_json((Option::<U128>::None, Option::<u64>::None))
            .view()
            .await?
            .json()?;
        assert_eq!(res, vec![token_1.clone(), token_2.clone(), token_3.clone()]);

        // Get tokens from_index=1, limit=None
        let res: Vec<Token> = mt.call("mt_tokens")
            .args_json((Some(U128(1)), Option::<u64>::None))
            .view()
            .await?
            .json()?;
        assert_eq!(res, vec![token_2.clone(), token_3.clone()]);

        // Get tokens from_index=None, limit=2
        let res: Vec<Token> = mt.call("mt_tokens")
            .args_json((Option::<U128>::None, Some(2u64)))
            .view()
            .await?
            .json()?;
        assert_eq!(res, vec![token_1.clone(), token_2.clone()]);

        // Get tokens from_index=2, limit=1
        let res: Vec<Token> = mt.call("mt_tokens")
            .args_json((Some(U128(2)), Some(1u64)))
            .view()
            .await?
            .json()?;
        assert_eq!(res, vec![token_3.clone()]);

        Ok(())
    }

    #[tokio::test]
    async fn simulate_enum_tokens_for_owner() -> anyhow::Result<()> {
        let worker = workspaces::sandbox().await?;
        let (mt, alice, _, defi) = init(&worker).await?;

        // Mint 5 tokens, alternating ownership between alice and the defi contract account.
        let token_1: Token = helper_mint(&mt, alice.id().clone(), 1000u128, "title1".to_string(), "desc1".to_string()).await?;
        helper_mint(&mt, defi.id().clone(), 20_000u128, "title2".to_string(), "desc2".to_string()).await?;
        let token_3: Token = helper_mint(&mt,  alice.id().clone(), 5u128, "title3".to_string(), "desc3".to_string()).await?;
        let token_4: Token = helper_mint(&mt, defi.id().clone(), 20_000u128, "title4".to_string(), "desc4".to_string()).await?;
        let token_5: Token = helper_mint(&mt, alice.id().clone(), 5u128, "title5".to_string(), "desc5".to_string()).await?;

        // Get all tokens for a specific owner, alice.
        let res: Vec<Token> = mt.call("mt_tokens_for_owner")
            .args_json((alice.id().clone(), Option::<U128>::None, Option::<u64>::None))
            .view()
            .await?
            .json()?;
        assert_eq!(res, vec![token_1.clone(), token_3.clone(), token_5.clone()]);

        // Get limit=None tokens at from_index=1 for defi account.
        let res: Vec<Token> = mt.call("mt_tokens_for_owner")
            .args_json((defi.id().clone(), Some(U128(1)), Option::<u64>::None))
            .view()
            .await?
            .json()?;
        assert_eq!(res, vec![token_4.clone()]);

        Ok(())
    }

}
