use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    log, near_bindgen,
};

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
struct Contract {}

trait T1 {
    fn foo(&self);
}

trait T2 {
    fn foo(&self);
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

impl Contract {
    fn bar(&self) {
        T1::foo(self);
        T2::foo(self);
    }
}

fn main() {}
