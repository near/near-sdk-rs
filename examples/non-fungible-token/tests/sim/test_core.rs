use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;
use near_sdk_sim::{call, view};

#[test]
fn simulate_simple_transfer() {
    let (root, nft, alice, _, _) = init();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());

    call!(
        root,
        nft.nft_transfer(
            alice.valid_account_id(),
            TOKEN_ID.into(),
            None,
            Some("simple transfer".to_string())
        ),
        deposit = 1
    )
    .assert_success();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, alice.account_id());
}

#[test]
fn simulate_transfer_call_fast_return_to_sender() {
    let (root, nft, _, receiver, _) = init();

    call!(
        root,
        nft.nft_transfer_call(
            receiver.valid_account_id(),
            TOKEN_ID.into(),
            None,
            Some("transfer & call".into()),
            "return-it-now".into()
        ),
        deposit = 1
    );

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());
}

#[test]
fn simulate_transfer_call_slow_return_to_sender() {
    let (root, nft, _, receiver, _) = init();

    call!(
        root,
        nft.nft_transfer_call(
            receiver.valid_account_id(),
            TOKEN_ID.into(),
            None,
            Some("transfer & call".into()),
            "return-it-later".into()
        ),
        deposit = 1
    );

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());
}

#[test]
fn simulate_transfer_call_fast_keep_with_sender() {
    let (root, nft, _, receiver, _) = init();

    call!(
        root,
        nft.nft_transfer_call(
            receiver.valid_account_id(),
            TOKEN_ID.into(),
            None,
            Some("transfer & call".into()),
            "keep-it-now".into()
        ),
        deposit = 1
    );

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, receiver.account_id());
}

#[test]
fn simulate_transfer_call_slow_keep_with_sender() {
    let (root, nft, _, receiver, _) = init();

    call!(
        root,
        nft.nft_transfer_call(
            receiver.valid_account_id(),
            TOKEN_ID.into(),
            None,
            Some("transfer & call".into()),
            "keep-it-later".into()
        ),
        deposit = 1
    );

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, receiver.account_id());
}

#[test]
fn simulate_transfer_call_receiver_panics() {
    let (root, nft, _, receiver, _) = init();

    call!(
        root,
        nft.nft_transfer_call(
            receiver.valid_account_id(),
            TOKEN_ID.into(),
            None,
            Some("transfer & call".into()),
            "incorrect message".into()
        ),
        deposit = 1
    );

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());
}
