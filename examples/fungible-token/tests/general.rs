/// Bring contract crate into namespace
extern crate fungible_token;

use std::convert::TryInto;

use defi::*;
/// Import the generated proxy contract
use fungible_token::ContractContract;

use near_sdk::{env, json_types::U128};
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

// Load in contract bytes
lazy_static_include::lazy_static_include_bytes! {
    TOKEN_WASM_BYTES => "res/fungible_token.wasm",
    DEFI_WASM_BYTES => "res/defi.wasm",
}

const REFERENCE: &str = "https://github.com/near/near-sdk-rs/tree/master/examples/fungible-token";
const REFERENCE_HASH: &str = "Aa4hsn9vdMetr2WDvYWduLCFpi6VZqJ3AzDm16VmSibG";

const FT_ID: &str = "ft";
const DEFI_ID: &str = "defi";

fn init(initial_balance: u128) -> (UserAccount, ContractAccount<ContractContract>, UserAccount) {
    let root = init_simulator(None);
    // uses default values for deposit and gas
    let ft = deploy!(
        // Contract Proxy
        contract: ContractContract,
        // Contract account id
        contract_id: FT_ID,
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: root,
        // init method
        init_method: new(
            root.account_id().try_into().unwrap(),
            initial_balance.into(),
            REFERENCE.to_string(),
            REFERENCE_HASH.try_into().unwrap()
        )
    );
    let alice = root.create_user("alice".to_string(), to_yocto("100"));
    register_user(&ft, &alice);

    (root, ft, alice)
}

// For given `contract` which uses the Account Storage standard,
// register the given `user`
fn register_user(contract: &ContractAccount<ContractContract>, user: &UserAccount) {
    call!(
        user,
        contract.storage_deposit(Some(user.account_id().try_into().unwrap())),
        deposit = env::storage_byte_cost() * 125
    )
    .assert_success();
}

#[test]
fn simulate_total_supply() {
    let initial_balance = to_yocto("100");
    let (_, ft, _) = init(initial_balance);

    let total_supply: U128 = view!(ft.ft_total_supply()).unwrap_json();

    assert_eq!(initial_balance, total_supply.0);
}

#[test]
fn simulate_simple_transfer() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (root, ft, alice) = init(initial_balance);

    // Transfer from root to alice.
    // Uses default gas amount, `near_sdk_sim::DEFAULT_GAS`
    call!(
        root,
        ft.ft_transfer(alice.account_id().try_into().unwrap(), transfer_amount.into(), None),
        deposit = 1
    )
    .assert_success();

    let root_balance: U128 =
        view!(ft.ft_balance_of(root.account_id().try_into().unwrap())).unwrap_json();
    let alice_balance: U128 =
        view!(ft.ft_balance_of(alice.account_id().try_into().unwrap())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount, root_balance.0);
    assert_eq!(transfer_amount, alice_balance.0);
}

#[test]
fn simulate_transfer_call_with_immediate_return_and_no_refund() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, _alice) = init(initial_balance);

    let defi = deploy!(
        contract: DeFiContract,
        contract_id: DEFI_ID,
        bytes: &DEFI_WASM_BYTES,
        signer_account: root
    );

    // defi contract must be registered as a FT account
    register_user(&ft, &defi.user_account);

    // root invests in defi by calling `ft_transfer_call`
    call!(
        root,
        ft.ft_transfer_call(
            DEFI_ID.try_into().unwrap(),
            transfer_amount.into(),
            None,
            "take-my-money".into()
        ),
        deposit = 1
    )
    .assert_success();

    let root_balance: U128 =
        view!(ft.ft_balance_of(root.account_id().try_into().unwrap())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(DEFI_ID.try_into().unwrap())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount, root_balance.0);
    assert_eq!(transfer_amount, defi_balance.0);
}

#[test]
fn simulate_transfer_call_when_called_contract_not_registered_with_ft() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, _alice) = init(initial_balance);

    deploy!(
        contract: DeFiContract,
        contract_id: DEFI_ID,
        bytes: &DEFI_WASM_BYTES,
        signer_account: root
    );

    // call fails because DEFI contract is not registered as FT user
    call!(
        root,
        ft.ft_transfer_call(
            DEFI_ID.try_into().unwrap(),
            transfer_amount.into(),
            None,
            "take-my-money".into()
        ),
        deposit = 1
    );

    // balances remain unchanged
    let root_balance: U128 =
        view!(ft.ft_balance_of(root.account_id().try_into().unwrap())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(DEFI_ID.try_into().unwrap())).unwrap_json();
    assert_eq!(initial_balance, root_balance.0);
    assert_eq!(0, defi_balance.0);
}

#[test]
fn simulate_transfer_call_with_promise_and_refund() {
    let transfer_amount = to_yocto("100");
    let refund_amount = to_yocto("50");
    let initial_balance = to_yocto("1000");
    let (root, ft, _alice) = init(initial_balance);

    let defi = deploy!(
        contract: DeFiContract,
        contract_id: DEFI_ID,
        bytes: &DEFI_WASM_BYTES,
        signer_account: root
    );
    register_user(&ft, &defi.user_account);

    call!(
        root,
        ft.ft_transfer_call(
            DEFI_ID.try_into().unwrap(),
            transfer_amount.into(),
            None,
            refund_amount.to_string()
        ),
        deposit = 1
    );

    let root_balance: U128 =
        view!(ft.ft_balance_of(root.account_id().try_into().unwrap())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(DEFI_ID.try_into().unwrap())).unwrap_json();
    assert_eq!(initial_balance - transfer_amount + refund_amount, root_balance.0);
    assert_eq!(transfer_amount - refund_amount, defi_balance.0);
}

#[test]
fn simulate_transfer_call_promise_panics_for_a_full_refund() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("1000");
    let (root, ft, _alice) = init(initial_balance);

    let defi = deploy!(
        contract: DeFiContract,
        contract_id: DEFI_ID,
        bytes: &DEFI_WASM_BYTES,
        signer_account: root
    );

    // defi contract must be registered as a FT account
    register_user(&ft, &defi.user_account);

    // root invests in defi by calling `ft_transfer_call`
    let res = call!(
        root,
        ft.ft_transfer_call(
            DEFI_ID.try_into().unwrap(),
            transfer_amount.into(),
            None,
            "no parsey as integer big panic oh no".to_string()
        ),
        deposit = 1
    );
    assert!(res.is_ok());

    // uncomment to see failure message from defi::value_please
    // println!("{:#?}", res.promise_results());

    let root_balance: U128 =
        view!(ft.ft_balance_of(root.account_id().try_into().unwrap())).unwrap_json();
    let defi_balance: U128 = view!(ft.ft_balance_of(DEFI_ID.try_into().unwrap())).unwrap_json();
    assert_eq!(initial_balance, root_balance.0);
    assert_eq!(0, defi_balance.0);
}
