use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount, DEFAULT_GAS,
    STORAGE_AMOUNT,
};
extern crate cross_contract_low_level;
use cross_contract_low_level::CrossContractContract;

near_sdk_sim::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/cross_contract_low_level.wasm").as_ref();
}

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<CrossContractContract>, UserAccount) {
    let master_account = init_simulator(None);
    let contract_account = deploy! {
        contract: CrossContractContract,
        contract_id: "contract",
        bytes: &TOKEN_WASM_BYTES,
        signer_account: master_account
    };
    let alice = master_account.create_user("alice".to_string(), initial_balance);
    (master_account, contract_account, alice)
}

#[test]
fn init_test() {
    let (_master_account, _contract_account, _alice) = init(to_yocto("10000"));
}

#[test]
fn check_promise() {
    let (master_account, contract, _alice) = init(to_yocto("10000"));
    let res = view!(contract.promise_checked());
    assert_eq!(res.from_json_value::<bool>().unwrap(), false);
    let status_id = "status".to_string();
    let status_amt = to_yocto("35");
    let res = call!(
        master_account,
        contract.deploy_status_message(status_id.clone(), status_amt.into()),
        STORAGE_AMOUNT,
        DEFAULT_GAS
    );
    let promise_outcomes = res.get_receipt_results();
    println!("{:#?}\n{:#?}", promise_outcomes, res);
    let message = "hello world";
    let res = call!(
        master_account,
        contract.complex_call(status_id.clone(), message.to_string()),
        gas = DEFAULT_GAS * 3
    );
    let value = res.get_json_value().unwrap();
    println!("COMPLEX CALL: {:#?}", res.promise_results());
    assert_eq!(view!(contract.promise_checked()).from_json_value::<bool>().unwrap(), true);
}

#[test]
fn test_sim_transfer() {
    // let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (master_account, contract, _alice) = init(initial_balance);
    let status_id = "status".to_string();
    let status_amt = to_yocto("35");
    let res = call!(
        master_account,
        contract.deploy_status_message(status_id.clone(), status_amt.into()),
        STORAGE_AMOUNT,
        DEFAULT_GAS
    );
    // let res = res.unwrap();
    // let promise_outcomes = runtime.get_receipt_outcomes(&res);
    // let ExecutionOutcome { status, .. } = res;
    let promise_outcomes = res.get_receipt_results();
    println!("{:#?}\n{:#?}", promise_outcomes, res);
    let message = "hello world";
    let res = call!(
        master_account,
        contract.complex_call(status_id.clone(), message.to_string()),
        gas = DEFAULT_GAS * 3
    );
    let value = res.get_json_value().unwrap();
    println!("COMPLEX CALL: {:#?}", res.promise_results());
    assert_eq!(message, value.to_string().trim_matches(|c| c == '"'));
    let v1: Vec<u8> = vec![42];
    let _v: Vec<u8> = vec![7, 1, 6, 5, 9, 255, 100, 11]; //, 2, 82, 13];
    let res = call!(master_account, contract.merge_sort(v1.clone()), gas = DEFAULT_GAS * 500);
    let value: Vec<u8> = res.from_json_value().unwrap();
    println!("{:#?}, {:#?}", value, res);
    assert_eq!(value, v1);
    let res = call!(master_account, contract.merge_sort(_v.clone()), gas = DEFAULT_GAS * 500);
    let outcomes = res.promise_results();
    print!("LAST_OUTCOMES: {:#?}", outcomes);
    // let res = res.unwrap();
    // let value = get_json_value(res.clone());
    // println!("{}, {:#?}", value.clone(), res);
    // let arr = near_sdk::serde_json::from_value::<Vec<u8>>(value).unwrap();
    // let (_last, b) = arr.iter().fold((0u8, true), |(prev, b), curr| (*curr, prev <= *curr && b));
    // assert!(b, "array is not sorted.");
    // let res = master_account.call(contract.merge_sort(_v.clone()), 0, DEFAULT_GAS);
    // println!("ERRORS: {:#?}", runtime.find_errors());
}
