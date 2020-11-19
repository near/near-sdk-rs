use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, ContractAccount, UserAccount, DEFAULT_GAS,
    STORAGE_AMOUNT,
};
extern crate cross_contract_high_level;
use cross_contract_high_level::CrossContractContract;

near_sdk_sim::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/cross_contract_high_level.wasm").as_ref();
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

    let promise_outcomes = res.get_receipt_results();
    // let ExecutionOutcome { status, .. } = res;
    println!("{:#?}\n{:#?}", promise_outcomes, &res);
    let message = "hello world";
    let res = call!(
        master_account,
        contract.complex_call(status_id.clone(), message.to_string()),
        gas = DEFAULT_GAS * 3
    );
    println!("{:#?}", res.clone());
    if !res.is_ok() {
        println!("{:#?}", res);
        assert!(false);
        return;
    }
    let value = res.unwrap_json_value();
    assert_eq!(message, value.to_string().trim_matches(|c| c == '"'));
    let v1: Vec<u8> = vec![42];
    let _v: Vec<u8> = vec![7, 1, 6, 5, 9, 255, 100, 11]; //, 2, 82, 13];
    let res = call!(master_account, contract.merge_sort(v1), gas = DEFAULT_GAS * 3);

    let value = res.unwrap_borsh::<Vec<u8>>();
    println!("{:#?}, {:#?}", value, res);
    let res = call!(master_account, contract.merge_sort(_v.clone()), gas = DEFAULT_GAS * 500);
    let arr = res.unwrap_borsh::<Vec<u8>>();
    println!("{:#?}, {:#?}", arr.clone(), res);
    let (_last, b) = arr.iter().fold((0u8, true), |(prev, b), curr| (*curr, prev <= *curr && b));
    assert!(b, "array is not sorted.");
}
