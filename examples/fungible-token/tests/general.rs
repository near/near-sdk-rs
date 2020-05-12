extern crate env_logger;
// #[war]
#[allow(unused_imports)]
#[macro_use]
extern crate log;
extern crate quickcheck;
#[macro_use(quickcheck)]
extern crate quickcheck_macros;
mod utils;

use near_primitives::types::Balance;
use utils::init_contract;

pub fn ntoy(near_amount: Balance) -> Balance {
    near_amount * 10u128.pow(24)
}

#[quickcheck]
fn qc_test_total_supply(initial_supply: Balance) -> bool {
    let deposit_amount = ntoy(initial_supply);
    let (mut runtime, root) = init_contract(deposit_amount);
    let total_supply = root.get_total_supply(&mut runtime);
    assert_eq!(total_supply, deposit_amount);
    assert_eq!(
        root.get_balance(&mut runtime, root.account_id().into()),
        deposit_amount
    );
    return true;
}

#[quickcheck]
fn qc_test_transfer(initial_supply: Balance) -> bool {
    let deposit_amount = ntoy(initial_supply);
    let (mut runtime, root) = init_contract(deposit_amount);
    let bob = root.create_external(&mut runtime, "bob".into(), ntoy(100));
    let transfer_amout = deposit_amount / 3;
    root.transfer(&mut runtime, bob.account_id().into(), transfer_amout);
    let total_supply = root.get_total_supply(&mut runtime);
    assert_eq!(total_supply, deposit_amount);
    assert_eq!(
        root.get_balance(&mut runtime, root.account_id().into()),
        deposit_amount - transfer_amout
    );
    assert_eq!(
        root.get_balance(&mut runtime, bob.account_id().into()),
        transfer_amout
    );
    return true;
}
