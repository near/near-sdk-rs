use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk_sim::{
    deploy_default, init_simulator, to_yocto, transaction::ExecutionOutcome, ContractAccount,
    UserAccount, DEFAULT_GAS, STORAGE_AMOUNT,
};
use std::str::FromStr;

extern crate fungible_token;
use fungible_token::FungibleTokenContract;
use near_sdk::PendingContractTx;

near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<FungibleTokenContract>, UserAccount) {
    let master_account = init_simulator(None);

    // default
    let contract_user = deploy_default!(
        // Contract Proxy
        FungibleTokenContract,
        // Contract account id
        "contract",
        // Bytes of contract
        &TOKEN_WASM_BYTES,
        // User deploying the contract,
        master_account,
        // init method
        new,
        // Args to initialize contract
        master_account.account_id(),
        initial_balance.into()
    );
    let alice = master_account.create_user("alice".to_string(), to_yocto("100"));
    (master_account, contract_user, alice)
}

#[test]
pub fn mint_token() {
    // let (runtime, alice, contract) = init_sim();
    // let master_account = init_simulator(None);
    // let balance: U128 = to_yocto("100000").into();
    // let initial_tx = PendingContractTx::new(
    //     "contract",
    //     "new",
    //     json!({
    //       "owner_id": root.account_id.clone(),
    //       "total_supply": balance
    //     }),
    //     false,
    // );
    // let contract = root.deploy_and_init(&TOKEN_WASM_BYTES, initial_tx);
    // let value = root.view(PendingContractTx::new(
    //     &contract.account_id,
    //     "get_total_supply",
    //     json!({}),
    //     true,
    // ));
    // let value: String = near_sdk::serde_json::from_value(value).unwrap();
    // assert_eq!(value, to_yocto("100000").to_string());
}

#[test]
fn test_sim_transfer() {
    let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (master_account, contract_account, alice) = init(initial_balance);
    let contract = contract_account.contract;
    let res = master_account.call(
        contract.transfer(alice.account_id.clone(), transfer_amount.into()),
        STORAGE_AMOUNT,
        DEFAULT_GAS,
    );
    use near_sdk_sim::transaction::ExecutionStatus::*;
    assert!(res.is_success());

    let value = master_account.view(contract.get_balance(master_account.account_id()));
    let value: String = near_sdk::serde_json::from_value(value).unwrap();
    let val = u128::from_str(&value).unwrap();
    assert_eq!(initial_balance - transfer_amount, val);
}
