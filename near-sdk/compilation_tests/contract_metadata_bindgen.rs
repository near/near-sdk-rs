use near_account_id::AccountIdRef;
use near_sdk::near_bindgen;

#[near_bindgen]
struct Contract {}

#[near_bindgen]
impl Contract {
    pub fn anything() {}
}

#[near_bindgen]
impl Contract {
    pub fn anything_else() {}
}

fn main() {
    let ext = Contract::ext(AccountIdRef::new_or_panic("0000").into());
    ext.contract_source_metadata();
}
