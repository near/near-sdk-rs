use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::token::Token;
use near_sdk_sim::{call, view};

#[test]
fn simulate_simple_transfer() {
    let (root, nft, alice, _) = init();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());

    call!(
        root,
        nft.nft_transfer(alice.valid_account_id(), TOKEN_ID.into(), None, None),
        deposit = 1
    )
    .assert_success();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, alice.account_id());
}
