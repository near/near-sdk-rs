use near_account_id::AccountIdRef;
use near_sdk::near;

#[near(contract_state)]
struct Contract {}

#[near]
impl Contract {
    pub fn anything() {}
}

#[near]
impl Contract {
    pub fn anything_else() {}
}

fn main() {
    let ext = Contract::ext(AccountIdRef::new_or_panic("0000").into());
    ext.contract_source_metadata();
}
