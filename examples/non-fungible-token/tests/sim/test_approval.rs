use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;
use near_sdk_sim::{call, view};
use std::collections::HashMap;

#[test]
fn simulate_simple_approve() {
    let (root, nft, alice, token_receiver, _) = init();

    // root approves alice
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), alice.valid_account_id(), None),
        deposit = 170000000000000000000
    )
    .assert_success();

    // check nft_is_approved, don't provide approval_id
    let alice_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), None)).unwrap_json();
    assert!(alice_approved);

    // check nft_is_approved, with approval_id=1
    let alice_approval_id_is_1: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), Some(1)))
            .unwrap_json();
    assert!(alice_approval_id_is_1);

    // check nft_is_approved, with approval_id=2
    let alice_approval_id_is_2: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), Some(2)))
            .unwrap_json();
    assert!(!alice_approval_id_is_2);

    // alternatively, one could check the data returned by nft_token
    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    let mut expected_approvals = HashMap::new();
    expected_approvals.insert(alice.account_id(), 1);
    assert_eq!(token.approved_account_ids.unwrap(), expected_approvals);

    // root approves alice again, which changes the approval_id and doesn't require as much deposit
    call!(root, nft.nft_approve(TOKEN_ID.into(), alice.valid_account_id(), None), deposit = 1)
        .assert_success();

    let alice_approval_id_is_2: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), Some(2)))
            .unwrap_json();
    assert!(alice_approval_id_is_2);

    // approving another account gives different approval_id
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), token_receiver.valid_account_id(), None),
        // note that token_receiver's account name is longer, and so takes more bytes to store and
        // therefore requires a larger deposit!
        deposit = 260000000000000000000
    )
    .assert_success();

    let token_receiver_approval_id_is_3: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), token_receiver.valid_account_id(), Some(3)))
            .unwrap_json();
    assert!(token_receiver_approval_id_is_3);
}

#[test]
fn simulate_approval_with_call() {
    let (root, nft, _, _, approval_receiver) = init();

    let outcome = call!(
        root,
        nft.nft_approve(
            TOKEN_ID.into(),
            approval_receiver.valid_account_id(),
            Some("return-now".to_string())
        ),
        deposit = 290000000000000000000
    );
    assert!(outcome.is_ok());
    let res: String = outcome.unwrap_json();
    assert_eq!("cool".to_string(), res);

    // Approve again; will set different approval_id (ignored by approval_receiver).
    // The approval_receiver implementation will return given `msg` after subsequent promise call,
    // if given something other than "return-now".
    let msg = "hahaha".to_string();
    let outcome = call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), approval_receiver.valid_account_id(), Some(msg.clone())),
        deposit = 1
    );
    assert!(outcome.is_ok());
    let res: String = outcome.unwrap_json();
    assert_eq!(msg, res);
}

#[test]
fn simulate_approved_account_transfers_token() {
    let (root, nft, alice, _, _) = init();

    // root approves alice
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), alice.valid_account_id(), None),
        deposit = 170000000000000000000
    )
    .assert_success();

    // alice sends to self
    call!(
        alice,
        nft.nft_transfer(
            alice.valid_account_id(),
            TOKEN_ID.into(),
            Some(1),
            Some("gotcha! bahahaha".to_string())
        ),
        deposit = 1
    )
    .assert_success();

    // token now owned by alice
    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, alice.account_id());
}

#[test]
fn simulate_revoke() {
    let (root, nft, alice, token_receiver, _) = init();

    // root approves alice
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), alice.valid_account_id(), None),
        deposit = 170000000000000000000
    )
    .assert_success();

    // root approves token_receiver
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), token_receiver.valid_account_id(), None),
        deposit = 260000000000000000000
    )
    .assert_success();

    // root revokes alice
    call!(root, nft.nft_revoke(TOKEN_ID.into(), alice.valid_account_id()), deposit = 1)
        .assert_success();

    // alice is revoked...
    let alice_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), None)).unwrap_json();
    assert!(!alice_approved);

    // but token_receiver is still approved
    let token_receiver_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), token_receiver.valid_account_id(), None))
            .unwrap_json();
    assert!(token_receiver_approved);

    // root revokes token_receiver
    call!(root, nft.nft_revoke(TOKEN_ID.into(), token_receiver.valid_account_id()), deposit = 1)
        .assert_success();

    // alice is still revoked...
    let alice_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), None)).unwrap_json();
    assert!(!alice_approved);

    // ...and now so is token_receiver
    let token_receiver_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), token_receiver.valid_account_id(), None))
            .unwrap_json();
    assert!(!token_receiver_approved);
}

#[test]
fn simulate_revoke_all() {
    let (root, nft, alice, token_receiver, _) = init();

    // root approves alice
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), alice.valid_account_id(), None),
        deposit = 170000000000000000000
    )
    .assert_success();

    // root approves token_receiver
    call!(
        root,
        nft.nft_approve(TOKEN_ID.into(), token_receiver.valid_account_id(), None),
        deposit = 260000000000000000000
    )
    .assert_success();

    // root revokes all
    call!(root, nft.nft_revoke_all(TOKEN_ID.into()), deposit = 1).assert_success();

    // alice is revoked...
    let alice_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), alice.valid_account_id(), None)).unwrap_json();
    assert!(!alice_approved);

    // but token_receiver is still approved
    let token_receiver_approved: bool =
        view!(nft.nft_is_approved(TOKEN_ID.into(), token_receiver.valid_account_id(), None))
            .unwrap_json();
    assert!(!token_receiver_approved);
}
