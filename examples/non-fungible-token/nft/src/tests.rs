use near_sdk::test_utils::{VMContextBuilder, accounts};
use near_sdk::{NearToken, testing_env};
use std::collections::HashMap;

use super::*;

const MINT_STORAGE_COST: NearToken = NearToken::from_yoctonear(5870000000000000000000u128);

fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(accounts(0))
        .signer_account_id(predecessor_account_id.clone())
        .predecessor_account_id(predecessor_account_id);
    builder
}

fn sample_token_metadata() -> TokenMetadata {
    TokenMetadata {
        title: Some("Olympus Mons".into()),
        description: Some("The tallest mountain in the charted solar system".into()),
        media: None,
        media_hash: None,
        copies: Some(1u64),
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    }
}

#[test]
fn test_new() {
    let mut context = get_context(accounts(1));
    testing_env!(context.build());
    let contract = Contract::new_default_meta(accounts(1).into());
    testing_env!(context.is_view(true).build());
    assert_eq!(contract.nft_token("1".to_string()), None);
}

#[test]
#[should_panic(expected = "The contract is not initialized")]
fn test_default() {
    let context = get_context(accounts(1));
    testing_env!(context.build());
    let _contract = Contract::default();
}

#[test]
fn test_mint() {
    let mut context = get_context(accounts(0));
    testing_env!(context.build());
    let mut contract = Contract::new_default_meta(accounts(0).into());

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build()
    );

    let token_id = "0".to_string();
    let token = contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());
    assert_eq!(token.token_id, token_id);
    assert_eq!(token.owner_id, accounts(0));
    assert_eq!(token.metadata.unwrap(), sample_token_metadata());
    assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
}

#[test]
fn test_transfer() {
    let mut context = get_context(accounts(0));
    testing_env!(context.build());
    let mut contract = Contract::new_default_meta(accounts(0).into());

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build()
    );
    let token_id = "0".to_string();
    contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build()
    );
    contract.nft_transfer(accounts(1), token_id.clone(), None, None);

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build()
    );
    if let Some(token) = contract.nft_token(token_id.clone()) {
        assert_eq!(token.token_id, token_id);
        assert_eq!(token.owner_id, accounts(1));
        assert_eq!(token.metadata.unwrap(), sample_token_metadata());
        assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
    } else {
        panic!("token not correctly created, or not found by nft_token");
    }
}

#[test]
fn test_approve() {
    let mut context = get_context(accounts(0));
    testing_env!(context.build());
    let mut contract = Contract::new_default_meta(accounts(0).into());

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build()
    );
    let token_id = "0".to_string();
    contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

    // alice approves bob
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(150000000000000000000u128))
            .predecessor_account_id(accounts(0))
            .build()
    );
    contract.nft_approve(token_id.clone(), accounts(1), None);

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build()
    );
    assert!(contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
}

#[test]
fn test_revoke() {
    let mut context = get_context(accounts(0));
    testing_env!(context.build());
    let mut contract = Contract::new_default_meta(accounts(0).into());

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build()
    );
    let token_id = "0".to_string();
    contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

    // alice approves bob
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(150000000000000000000))
            .predecessor_account_id(accounts(0))
            .build()
    );
    contract.nft_approve(token_id.clone(), accounts(1), None);

    // alice revokes bob
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build()
    );
    contract.nft_revoke(token_id.clone(), accounts(1));
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build()
    );
    assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), None));
}

#[test]
fn test_revoke_all() {
    let mut context = get_context(accounts(0));
    testing_env!(context.build());
    let mut contract = Contract::new_default_meta(accounts(0).into());

    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build()
    );
    let token_id = "0".to_string();
    contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

    // alice approves bob
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(150000000000000000000))
            .predecessor_account_id(accounts(0))
            .build()
    );
    contract.nft_approve(token_id.clone(), accounts(1), None);

    // alice revokes bob
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(accounts(0))
            .build()
    );
    contract.nft_revoke_all(token_id.clone());
    testing_env!(
        context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(NearToken::from_near(0))
            .build()
    );
    assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
}
