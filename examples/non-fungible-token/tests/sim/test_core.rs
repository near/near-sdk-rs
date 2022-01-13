use crate::utils::{init, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;
use near_sdk::serde_json::{self, json};
use near_sdk_sim::{call, view};

#[test]
fn simulate_simple_transfer() {
    let (root, nft, alice, _, _) = init();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());

    call!(
        root,
        nft.nft_transfer(
            alice.account_id(),
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
            receiver.account_id(),
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
            receiver.account_id(),
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
            receiver.account_id(),
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
            receiver.account_id(),
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
            receiver.account_id(),
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

#[test]
fn simulate_transfer_call_receiver_panics_and_nft_resolve_transfer_produces_log() {
    let (root, nft, _, receiver, _) = init();
    let args = json!({
      "receiver_id": receiver.account_id(),
      "token_id": TOKEN_ID,
      "memo": Some("transfer & call"),
      "msg": "incorrect message"

    });
    let args = serde_json::to_vec(&args).unwrap();
    let res = root
        .create_transaction(nft.account_id())
        .function_call("nft_transfer_call".to_string(), args, 35_000_000_000_000 + 1, 1)
        .submit();

    // Prints final log
    assert_eq!(res.logs().len(), 1);

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());
}

#[test]
fn simulate_transfer_call_receiver_panics_and_nft_resolve_transfer_produces_no_log_if_not_enough_gas() {
    let (root, nft, _, receiver, _) = init();
    let args = json!({
      "receiver_id": receiver.account_id(),
      "token_id": TOKEN_ID,
      "memo": Some("transfer & call"),
      "msg": "incorrect message"

    });
    let args = serde_json::to_vec(&args).unwrap();
    let res = root
        .create_transaction(nft.account_id())
        .function_call("nft_transfer_call".to_string(), args, 35_000_000_000_000, 1)
        .submit();

    // Prints final log
    assert_eq!(res.logs().len(), 0);

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());
}

#[test]
fn simulate_transfer_call_no_extra_log() {
    let (root, nft, _, receiver, _) = init();

    let args = json!({
      "receiver_id": receiver.account_id(),
      "token_id": TOKEN_ID,
      "memo": Some("transfer & call"),
      "msg": "keep-it-now"

    });
    let args = serde_json::to_vec(&args).unwrap();
    let res = root
        .create_transaction(nft.account_id())
        .function_call("nft_transfer_call".to_string(), args, 200_000_000_000_000, 1)
        .submit();

    assert_eq!(res.logs().len(), 0);
    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, receiver.account_id());
}

#[test]
fn simulate_simple_transfer_logs() {
    let (root, nft, alice, _, _) = init();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());

    let args = json!({
      "receiver_id": alice.account_id(),
      "token_id": TOKEN_ID,
      "memo": Some("simple transfer"),
    });
    let args = serde_json::to_vec(&args).unwrap();
    let res = root
        .create_transaction(nft.account_id())
        .function_call("nft_transfer".to_string(), args, 200_000_000_000_000, 1)
        .submit();

    assert_eq!(res.logs().len(), 1);

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, alice.account_id());
}

#[test]
fn simulate_simple_transfer_no_logs_on_failure() {
    let (root, nft, _, _, _) = init();

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());

    let args = json!({
      // transfer to the current owner should fail and not print log
      "receiver_id": root.account_id(),
      "token_id": TOKEN_ID,
      "memo": Some("simple transfer"),
    });
    let args = serde_json::to_vec(&args).unwrap();
    let res = root
        .create_transaction(nft.account_id())
        .function_call("nft_transfer".to_string(), args, 200_000_000_000_000, 1)
        .submit();

    assert_eq!(res.logs().len(), 0);

    let token: Token = view!(nft.nft_token(TOKEN_ID.into())).unwrap_json();
    assert_eq!(token.owner_id, root.account_id());
}
