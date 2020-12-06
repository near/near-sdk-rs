use near_sdk_sim::{
    call, deploy, init_simulator, near_crypto::Signer, to_yocto, view, ContractAccount,
    UserAccount, STORAGE_AMOUNT,
};
use std::str::FromStr;

/// Bring contract crate into namespace
extern crate collection_comparison;
/// Import the generated proxy contract
/// Magic function??
use collection_comparison::CollectionsContract;
use near_sdk::json_types::U128;
use near_sdk_sim::account::AccessKey;

/// Load in contract bytes
near_sdk_sim::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/collection_comparison.wasm").as_ref();
}

fn init(initial_balance: u128) -> (UserAccount, ContractAccount<CollectionsContract>, UserAccount) {
    // todo: useful comment here
    let master_account = init_simulator(None);
    // uses default values for deposit and gas
    let contract_user = deploy!(
        // Contract Proxy
        contract: CollectionsContract,
        // Contract account id
        contract_id: "contract",
        // Bytes of contract
        bytes: &TOKEN_WASM_BYTES,
        // User deploying the contract,
        signer_account: master_account,
        // init method
        init_method: new()
    );
    let alice = master_account.create_user("alice".to_string(), to_yocto("100"));
    (master_account, contract_user, alice)
}

/// Example of how to create and use an user transaction.
fn init2(initial_balance: u128) {
    let master_account = init_simulator(None);
    let txn = master_account.create_transaction("contract".into());
    // uses default values for deposit and gas
    let res = txn
        .create_account()
        .add_key(master_account.signer.public_key(), AccessKey::full_access())
        .transfer(initial_balance)
        .deploy_contract((&TOKEN_WASM_BYTES).to_vec())
        .submit();
    println!("{:#?}", res);
}

#[test]
pub fn mint_token() {
    init2(to_yocto("35"));
}

// Let's not even worry about this test until the ez one passes
// #[test]
// fn test_sim_transfer() {
//     let transfer_amount = to_yocto("100");
//     let initial_balance = to_yocto("100000");
//     let (master_account, contract, alice) = init(initial_balance);
//     /// Uses default gas amount, `near_sdk_sim::DEFAULT_GAS` 300_000_000_000_000
//     let res = call!(
//         master_account,
//         contract.add_tree_map("mykey".to_string(), vec![19, 31]),
//         deposit = STORAGE_AMOUNT
//     );
//     println!("{:#?}", res.status());
//     assert!(res.is_ok());
//
//     let value = view!(contract.get_tree_map("mykey".to_string()));
//     // let value: U128 = value.unwrap_json();
//     assert_eq!(vec![19, 31], value);
// }
