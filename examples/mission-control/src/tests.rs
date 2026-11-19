use near_sdk::env;

use crate::account::Quantity;
use crate::asset::Asset;
use crate::mission_control::MissionControl;

#[test]
fn add_agent() {
    let account_id = env::signer_account_id();

    let mut contract = MissionControl::default();
    contract.add_agent();
    assert_eq!(Some(true), contract.simulate(account_id.clone()));
    assert_eq!(Some(Quantity(2)), contract.assets_quantity(account_id.clone(), Asset::MissionTime));
}
