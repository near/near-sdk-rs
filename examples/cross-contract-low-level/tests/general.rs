use near_sdk::AccountId;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount, DEFAULT_GAS,
    STORAGE_AMOUNT,
};

extern crate cross_contract_low_level;
// Note: the struct xxxxxxContract is created by #[near_bindgen] from near-sdk in combination with
// near-sdk-sim
use cross_contract_low_level::CrossContractContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TOKEN_WASM_BYTES => "res/cross_contract_low_level.wasm",
}

fn init(
    initial_balance: u128,
) -> (UserAccount, ContractAccount<CrossContractContract>, UserAccount) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_price = 0;
    genesis.gas_limit = u64::MAX;
    let master_account = init_simulator(Some(genesis));
    let contract_account = deploy! {
        contract: CrossContractContract,
        contract_id: "contract",
        bytes: &TOKEN_WASM_BYTES,
        signer_account: master_account
    };
    let alice =
        master_account.create_user(AccountId::new_unchecked("alice".to_string()), initial_balance);
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
    assert_eq!(res.unwrap_json::<bool>(), false);
    let status_id: near_sdk::AccountId = "status".parse().unwrap();
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
    println!("COMPLEX CALL: {:#?}", res.promise_results());
    assert_eq!(view!(contract.promise_checked()).unwrap_json::<bool>(), true);
}

#[test]
fn test_sim_transfer() {
    // let transfer_amount = to_yocto("100");
    let initial_balance = to_yocto("100000");
    let (master_account, contract, _alice) = init(initial_balance);
    let status_id: near_sdk::AccountId = "status".parse().unwrap();
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
    let value = res.unwrap_json_value();
    println!("COMPLEX CALL: {:#?}", res.promise_results());
    assert_eq!(message, value.to_string().trim_matches(|c| c == '"'));
    let v1: Vec<u8> = vec![42];
    let _v: Vec<u8> = vec![7, 1, 6, 5, 9, 255, 100, 11]; //, 2, 82, 13];
    let res = call!(master_account, contract.merge_sort(v1.clone()), gas = DEFAULT_GAS * 500);
    let value: Vec<u8> = res.unwrap_json();
    println!("{:#?}, {:#?}", value, res);
    assert_eq!(value, v1);
    let res = call!(master_account, contract.merge_sort(_v.clone()), gas = DEFAULT_GAS * 500);
    let outcomes = res.promise_results();
    print!("LAST_OUTCOMES: {:#?}", outcomes);
    let arr = res.unwrap_json::<Vec<u8>>();
    let (_last, b) = arr.iter().fold((0u8, true), |(prev, b), curr| (*curr, prev <= *curr && b));
    assert!(b, "array is not sorted.");
    let res = call!(master_account, contract.merge_sort(_v.clone()));
    assert!(!res.is_ok(), "Must fail because too little gas.");
    assert!(!res.promise_errors().is_empty(), "At least one promise must fail.");
    println!("ERRORS: {:#?}", res.promise_errors());
}
