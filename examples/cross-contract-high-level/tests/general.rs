use near_sdk_sim::{
    deploy_default, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS,
    STORAGE_AMOUNT,
};
extern crate cross_contract_high_level;
use cross_contract_high_level::CrossContractContract;

near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/cross_contract_high_level.wasm").as_ref();
}

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<CrossContractContract>, UserAccount) {
    let master_account = init_simulator(None);
    let contract_account =
        deploy_default!(CrossContractContract, "contract", &TOKEN_WASM_BYTES, master_account);
    let alice = master_account.create_user("alice".to_string(), initial_balance);
    (master_account, contract_account, alice)
}

#[test]
fn test_sim_transfer() {
    // let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (master_account, contract_user, _alice) = init(initial_balance);
    let status_id = "status".to_string();
    let status_amt = to_yocto("35");
    let contract = contract_user.contract;
    let res = master_account.call(
        contract.deploy_status_message(status_id.clone(), status_amt.into()),
        STORAGE_AMOUNT,
        DEFAULT_GAS,
    );

    let promise_outcomes = res.get_receipt_outcomes();
    // let ExecutionOutcome { status, .. } = res;
    println!("{:#?}\n{:#?}", promise_outcomes, &res);
    let message = "hello world";
    let res = master_account.call(
        contract.complex_call(status_id.clone(), message.to_string()),
        0,
        DEFAULT_GAS * 3,
    );
    println!("{:#?}", res.clone());
    if !res.is_ok() {
        println!("{:#?}", res);
        assert!(false);
        return;
    }
    let value = res.get_json_value().unwrap();
    assert_eq!(message, value.to_string().trim_matches(|c| c == '"'));
    let v1: Vec<u8> = vec![42];
    let _v: Vec<u8> = vec![7, 1, 6, 5, 9, 255, 100, 11]; //, 2, 82, 13];
    let res = master_account.call(contract.merge_sort(v1), 0, DEFAULT_GAS * 3);

    let value = res.get_borsh_value::<Vec<u8>>().unwrap();
    println!("{:#?}, {:#?}", value, res);
    let res = master_account.call(contract.merge_sort(_v.clone()), 0, DEFAULT_GAS * 500);
    let arr = res.get_borsh_value::<Vec<u8>>().unwrap();
    println!("{:#?}, {:#?}", arr.clone(), res);
    let (_last, b) = arr.iter().fold((0u8, true), |(prev, b), curr| (*curr, prev <= *curr && b));
    assert!(b, "array is not sorted.");
}
