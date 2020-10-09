use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk_sim::test_runtime::{init_test_runtime, to_yocto, PendingContractTx};
use near_sdk_sim::{transaction::ExecutionOutcome, TestRuntime, User, DEFAULT_GAS, STORAGE_AMOUNT};
use std::str::FromStr;

extern crate fungible_token;
use fungible_token::FungibleTokenContract;

near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/fungible_token.wasm").as_ref();
}
//
// Example of how the created contract
//
// struct FungibleTokenContract {
//     account_id: AccountId,
// }
//
// impl FungibleTokenContract {
//     pub fn new(&self, owner_id: AccountId, total_supply: Balance) -> PendingContractTx {
//         let balance: U128 = total_supply.into();
//         PendingContractTx::new(
//             &self.account_id,
//             "new",
//             json!({
//               "owner_id": owner_id,
//               "total_supply": balance
//             }),
//             false,
//         )
//     }
// }

fn init(initial_balance: u128) -> (TestRuntime, User, User) {
    let runtime = init_test_runtime();
    let root = runtime.get_root();
    // let balance: U128 = initial_balance.into();
    let contract = FungibleTokenContract { account_id: "contract".to_string() };
    let contract = root.deploy_and_init(
        &TOKEN_WASM_BYTES,
        contract.new(root.account_id.clone(), initial_balance.into()),
    );
    let alice = runtime.create_user("alice".to_string(), to_yocto("100"));
    (runtime, contract, alice)
}

#[test]
pub fn mint_token() {
    // let (runtime, alice, contract) = init_sim();
    let runtime = init_test_runtime();
    let root = runtime.get_root();
    let balance: U128 = to_yocto("100000").into();
    let initial_tx = PendingContractTx::new(
        "contract",
        "new",
        json!({
          "owner_id": root.account_id.clone(),
          "total_supply": balance
        }),
        false,
    );
    let contract = root.deploy_and_init(&TOKEN_WASM_BYTES, initial_tx);
    let value = contract.view(PendingContractTx::new(
        &contract.account_id,
        "get_total_supply",
        json!({}),
        true,
    ));
    let value: String = near_sdk::serde_json::from_value(value).unwrap();
    assert_eq!(value, to_yocto("100000").to_string());
}

#[test]
fn test_sim_transfer() {
    let transfer_amount = to_yocto("100");
    let inital_balance = to_yocto("100000");
    let (runtime, contract, alice) = init(inital_balance);
    let root = runtime.get_root();
    let res = root.call(
        PendingContractTx::new(
            &contract.account_id,
            &"transfer",
            json!({
            "new_owner_id": alice.account_id.clone(),
            "amount": transfer_amount.to_string()
            }),
            false,
        ),
        STORAGE_AMOUNT,
        DEFAULT_GAS,
    );
    let res = res.unwrap();
    use near_sdk_sim::transaction::ExecutionStatus::*;
    let ExecutionOutcome { status, .. } = res;
    match status {
        SuccessValue(_) => (),
        other => panic!("status not SucessValue, instead {:?}", other),
    }
    let value = root.view(PendingContractTx::new(
        &contract.account_id,
        &"get_balance",
        json!({
            "owner_id": "root".to_string()
        }),
        true,
    ));
    let value: String = near_sdk::serde_json::from_value(value).unwrap();
    let val = u128::from_str(&value).unwrap();
    assert_eq!(inital_balance - transfer_amount, val);
}
