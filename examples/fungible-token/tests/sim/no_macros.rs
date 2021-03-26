use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk_sim::{to_yocto, DEFAULT_GAS};

use crate::utils::init_no_macros as init;

#[test]
fn simulate_total_supply() {
    let initial_balance = to_yocto("100");
    let (_, ft, _) = init(initial_balance);

    let total_supply: U128 = ft.view(ft.account_id(), "ft_total_supply", b"").unwrap_json();

    assert_eq!(initial_balance, total_supply.0);
}

#[test]
fn simulate_simple_transfer() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (root, ft, alice) = init(initial_balance);

    // Transfer from root to alice.
    root.call(
        ft.account_id(),
        "ft_transfer",
        &json!({
            "receiver_id": alice.valid_account_id(),
            "amount": U128::from(transfer_amount)
        })
        .to_string()
        .into_bytes(),
        DEFAULT_GAS,
        1, // deposit
    )
    .assert_success();

    let root_balance: U128 = root
        .view(
            ft.account_id(),
            "ft_balance_of",
            &json!({
                "account_id": root.valid_account_id()
            })
            .to_string()
            .into_bytes(),
        )
        .unwrap_json();
    let alice_balance: U128 = alice
        .view(
            ft.account_id(),
            "ft_balance_of",
            &json!({
                "account_id": alice.valid_account_id()
            })
            .to_string()
            .into_bytes(),
        )
        .unwrap_json();
    assert_eq!(initial_balance - transfer_amount, root_balance.0);
    assert_eq!(transfer_amount, alice_balance.0);
}
