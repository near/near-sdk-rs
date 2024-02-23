use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, log, near_bindgen,
    test_utils::get_logs,
    PanicOnDefault,
};

trait T1 {
    fn foo(&self);
}

trait T2 {
    fn foo(&self);
}

trait T3 {
    fn contract_source_metadata(&self);
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {}

// here to make sure the method does not collide with the serialize method impl from borsh
#[near_bindgen]
impl Contract {
    pub fn serialize(&mut self) {
        log!("serialize");
    }
}

#[near_bindgen]
impl T1 for Contract {
    fn foo(&self) {
        log!("foo_one")
    }
}

#[near_bindgen]
impl T2 for Contract {
    #[abi_alias("foo_two")]
    fn foo(&self) {
        log!("foo_two")
    }
}

#[near_bindgen]
impl T3 for Contract {
    #[abi_alias("t3_contract_source_metadata")]
    fn contract_source_metadata(&self) {
        log!("contract_source_metadata from T3")
    }
}

#[test]
fn test_serialize_method() {
    let mut contract = Contract {};
    Contract::serialize(&mut contract);
    let logs = get_logs();
    assert_eq!(logs[0], "serialize");
}

#[test]
fn test_aliased_method() {
    let contract = Contract {};
    T1::foo(&contract);
    T2::foo(&contract);
    let logs = get_logs();

    assert_eq!(logs[0], "foo_one");
    assert_eq!(logs[1], "foo_two");

    // making sure the method T2::foo exists in abi as foo_two
    let _ = Contract::ext(env::current_account_id()).foo_two();
    // also making sure the method T1::foo exists in abi as foo
    let _ = Contract::ext(env::current_account_id()).foo();
}

#[test]
fn test_reserved_method_name() {
    // making sure the method T3::contract_source_metadata exists in abi as t3_contract_source_metadata
    let _ = Contract::ext(env::current_account_id()).t3_contract_source_metadata();
    // making sure contract_source_metadata is also available
    let _ = Contract::ext(env::current_account_id()).contract_source_metadata();
}
