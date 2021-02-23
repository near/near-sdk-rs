/// Bring contract crate into namespace
extern crate fungible_token;

use std::convert::TryInto;

/// Import the generated proxy contract
use fungible_token::ContractContract;

use near_sdk::{env, json_types::U128};
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

// Load in contract bytes
near_sdk_sim::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}

fn init(initial_balance: u128) -> (UserAccount, ContractAccount<ContractContract>, UserAccount) {
    let master_account = init_simulator(None);
    // uses default values for deposit and gas
    let contract_user = deploy!(
        // Contract Proxy
        contract: ContractContract,
        // Contract account id
        contract_id: "contract",
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: master_account,
        // init method
        init_method: new(master_account.account_id().try_into().unwrap(), initial_balance.into())
    );
    let alice = master_account.create_user("alice".to_string(), to_yocto("100"));
    (master_account, contract_user, alice)
}

#[test]
fn test_sim_transfer() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (master_account, contract, alice) = init(initial_balance);

    // Register `alice` account first with required deposit.
    call!(
        master_account,
        contract.storage_deposit(Some(alice.account_id().try_into().unwrap())),
        deposit = env::storage_byte_cost() * 125
    )
    .assert_success();

    // Transfer from master to alice.
    // Uses default gas amount, `near_sdk_sim::DEFAULT_GAS`
    let res = call!(
        master_account,
        contract.ft_transfer(alice.account_id().try_into().unwrap(), transfer_amount.into(), None),
        deposit = 1
    );
    println!("{:#?}\n Cost:\n{:#?}", res.status(), res.profile_data());
    assert!(res.is_ok());

    // Check master's balance deducted sent funds.
    let value = view!(contract.ft_balance_of(master_account.account_id().try_into().unwrap()));
    let value: U128 = value.unwrap_json();
    assert_eq!(initial_balance - transfer_amount, value.0);
}
