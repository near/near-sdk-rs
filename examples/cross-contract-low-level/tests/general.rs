use near_sdk_sim::test_runtime::{init_test_runtime, to_yocto};
use near_sdk_sim::{get_value, TestRuntime, User, DEFAULT_GAS, STORAGE_AMOUNT};
extern crate cross_contract_low_level;
use cross_contract_low_level::CrossContractContract;

near_sdk_sim::lazy_static::lazy_static! {
    static ref TOKEN_WASM_BYTES: &'static [u8] = include_bytes!("../res/cross_contract_low_level.wasm").as_ref();
}

fn init(initial_balance: u128) -> (TestRuntime, User, User, CrossContractContract) {
    let runtime = init_test_runtime(None);
    let root = runtime.get_root();
    // let balance: U128 = initial_balance.into();
    let contract = CrossContractContract { account_id: "contract".to_string() };
    let contract_user = root.deploy(&TOKEN_WASM_BYTES, "contract".to_string());
    let alice = runtime.create_user("alice".to_string(), initial_balance);
    (runtime, contract_user, alice, contract)
}

#[test]
fn test_sim_transfer() {
    // let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (runtime, _contract_user, _alice, contract) = init(initial_balance);
    let root = runtime.get_root();
    let status_id = "status".to_string();
    let status_amt = to_yocto("35");
    let res = root.call(
        contract.deploy_status_message(status_id.clone(), status_amt.into()),
        STORAGE_AMOUNT,
        DEFAULT_GAS,
    );
    let res = res.unwrap();
    let promise_outcome = runtime.get_outcome(&res.receipt_ids[0]);
    // let ExecutionOutcome { status, .. } = res;
    println!("{:?}\n{:?}", promise_outcome, res);
    let message = "hello world";
    let res = root.call(
        contract.complex_call(status_id.clone(), message.to_string()),
        0,
        DEFAULT_GAS * 3,
    );
    let value = get_value(res.unwrap());
    assert_eq!(message, value.to_string().trim_matches(|c| c == '"'));
    let v1: Vec<u8> = vec![42];
    let _v: Vec<u8> = vec![1, 5, 6, 9, 10, 100, 111, 82, 13, 199, 200, 0];
    let res = root.call(contract.merge_sort(v1), 0, DEFAULT_GAS * 3);
    let value = get_value(res.unwrap());
    println!("{}", value);
    let res = root.call(contract.merge_sort(_v), 0, DEFAULT_GAS * 3);
    let value = get_value(res.unwrap());
    println!("{}", value);
}
