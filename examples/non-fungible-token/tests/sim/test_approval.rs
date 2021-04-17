use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::token::Token;
use near_sdk_sim::{call, view};
use std::collections::HashMap;

#[test]
fn simulate_simple_approve() {
    let (root, nft, alice, _) = init();

    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), alice.valid_account_id(), None),
        deposit = 170000000000000000000
    )
    .assert_success();

    let alice_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), None)).unwrap_json();
    assert!(alice_approved);

    let alice_approval_id_is_1: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), Some(1)))
            .unwrap_json();
    assert!(alice_approval_id_is_1);

    let alice_approval_id_is_2: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), Some(2)))
            .unwrap_json();
    assert!(!alice_approval_id_is_2);

    // alternatively, you could check the data returned by nft_token
    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    let mut expected_approvals = HashMap::new();
    expected_approvals.insert(alice.account_id(), 1);
    assert_eq!(token.approved_account_ids.unwrap(), expected_approvals);
}
