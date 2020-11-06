use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount, STORAGE_AMOUNT,
};
use std::str::FromStr;

extern crate fungible_token;
use fungible_token::FungibleTokenContract;

near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<FungibleTokenContract>, UserAccount) {
    println!("let's start");
    let master_account = init_simulator(None);
    // uses default values for deposit and gas
    let contract_user = deploy!(
        // Contract Proxy
        contract: FungibleTokenContract,
        // Contract account id
        contract_id: "contract",
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: master_account,
        // init method
        init_method: new(master_account.account_id(), initial_balance.into())
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
    let (master_account, contract, alice) = init(initial_balance);
    let res = call!(
        master_account,
        contract.transfer(alice.account_id.clone(), transfer_amount.into()),
        deposit = STORAGE_AMOUNT
    );
    println!("{:#?}", res.status());
    assert!(res.is_ok());

    let value = view!(contract.get_balance(master_account.account_id()));
    let value: String = value.from_json_value().unwrap();
    let val = u128::from_str(&value).unwrap();
    assert_eq!(initial_balance - transfer_amount, val);
}
