use crate::utils::{helper_mint, init};
use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::U128;
use near_sdk_sim::{view, ContractAccount, UserAccount};
use non_fungible_token::ContractContract as NftContract;

fn mint_more(root: &UserAccount, nft: &ContractAccount<NftContract>) {
    helper_mint(
        "1".to_string(),
        &root,
        &nft,
        "Black as the Night".to_string(),
        "In charcoal".to_string(),
    );
    helper_mint(
        "2".to_string(),
        &root,
        &nft,
        "Hamakua".to_string(),
        "Vintage recording".to_string(),
    );
    helper_mint(
        "3".to_string(),
        &root,
        &nft,
        "Aloha ke akua".to_string(),
        "Original with piano".to_string(),
    );
}

#[test]
fn simulate_enum_total_supply() {
    let (root, nft, _, _, _) = init();
    mint_more(&root, &nft);

    let total_supply: U128 = view!(nft.nft_total_supply()).unwrap_json();
    assert_eq!(total_supply, U128::from(4));
}

#[test]
fn simulate_enum_nft_tokens() {
    let (root, nft, _, _, _) = init();
    mint_more(&root, &nft);

    // No optional args should return all
    let mut tokens: Vec<Token> = view!(nft.nft_tokens(None, None)).unwrap_json();
    assert_eq!(tokens.len(), 4);
    // Start at "1", with no limit arg
    tokens = view!(nft.nft_tokens(Some(U128::from(1)), None)).unwrap_json();
    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens.get(0).unwrap().token_id, "1".to_string());
    assert_eq!(tokens.get(1).unwrap().token_id, "2".to_string());
    assert_eq!(tokens.get(2).unwrap().token_id, "3".to_string());

    // Start at "2", with limit 1
    tokens = view!(nft.nft_tokens(Some(U128::from(2)), Some(1u64))).unwrap_json();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens.get(0).unwrap().token_id, "2".to_string());

    // Don't specify from_index, but limit 2
    tokens = view!(nft.nft_tokens(None, Some(2u64))).unwrap_json();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens.get(0).unwrap().token_id, "0".to_string());
    assert_eq!(tokens.get(1).unwrap().token_id, "1".to_string());
}

#[test]
fn simulate_enum_nft_supply_for_owner() {
    let (root, nft, alice, _, _) = init();

    // Get number from account with no NFTs
    let mut owner_num_tokens: U128 =
        view!(nft.nft_supply_for_owner(alice.valid_account_id())).unwrap_json();
    assert_eq!(owner_num_tokens, U128::from(0));

    owner_num_tokens = view!(nft.nft_supply_for_owner(root.valid_account_id())).unwrap_json();
    assert_eq!(owner_num_tokens, U128::from(1));

    mint_more(&root, &nft);

    owner_num_tokens = view!(nft.nft_supply_for_owner(root.valid_account_id())).unwrap_json();
    assert_eq!(owner_num_tokens, U128::from(4));
}

#[test]
fn simulate_enum_nft_tokens_for_owner() {
    let (root, nft, alice, _, _) = init();
    mint_more(&root, &nft);

    // Get tokens from account with no NFTs
    let mut owner_tokens: Vec<Token> =
        view!(nft.nft_tokens_for_owner(alice.valid_account_id(), None, None)).unwrap_json();
    assert_eq!(owner_tokens.len(), 0);

    // Get tokens with no optional args
    owner_tokens =
        view!(nft.nft_tokens_for_owner(root.valid_account_id(), None, None)).unwrap_json();
    assert_eq!(owner_tokens.len(), 4);

    // With from_index and no limit
    owner_tokens =
        view!(nft.nft_tokens_for_owner(root.valid_account_id(), Some(U128::from(2)), None))
            .unwrap_json();
    assert_eq!(owner_tokens.len(), 2);
    assert_eq!(owner_tokens.get(0).unwrap().token_id, "2".to_string());
    assert_eq!(owner_tokens.get(1).unwrap().token_id, "3".to_string());

    // With from_index and limit 1
    owner_tokens =
        view!(nft.nft_tokens_for_owner(root.valid_account_id(), Some(U128::from(1)), Some(1)))
            .unwrap_json();
    assert_eq!(owner_tokens.len(), 1);
    assert_eq!(owner_tokens.get(0).unwrap().token_id, "1".to_string());

    // No from_index but limit 3
    owner_tokens =
        view!(nft.nft_tokens_for_owner(root.valid_account_id(), None, Some(3))).unwrap_json();
    assert_eq!(owner_tokens.len(), 3);
    assert_eq!(owner_tokens.get(0).unwrap().token_id, "0".to_string());
    assert_eq!(owner_tokens.get(1).unwrap().token_id, "1".to_string());
    assert_eq!(owner_tokens.get(2).unwrap().token_id, "2".to_string());
}
