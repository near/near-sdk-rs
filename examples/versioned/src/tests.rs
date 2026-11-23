use near_sdk::test_utils::VMContextBuilder;
use near_sdk::test_utils::test_env::{alice, bob};
use near_sdk::testing_env;

use super::*;

fn set_predecessor_and_deposit(predecessor: AccountId, deposit: NearToken) {
    testing_env!(
        VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .attached_deposit(deposit)
            .build()
    )
}

#[test]
fn basic() {
    let mut contract = VersionedContract::default();
    set_predecessor_and_deposit(bob(), NearToken::from_yoctonear(8));
    contract.deposit();

    set_predecessor_and_deposit(alice(), NearToken::from_yoctonear(10));
    contract.deposit();

    set_predecessor_and_deposit(bob(), NearToken::from_yoctonear(20));
    contract.deposit();

    assert_eq!(contract.get_deposit(&alice()), Some(&NearToken::from_yoctonear(10)));
    assert_eq!(contract.get_deposit(&bob()), Some(&NearToken::from_yoctonear(28)));
    assert_eq!(contract.get_nonce(), 3);
}

#[test]
fn contract_v0_interactions() {
    let mut contract = {
        let mut funders = IterableMap::new(b"f");
        funders.insert(bob(), NearToken::from_yoctonear(8));
        VersionedContract::V0(ContractV0 { funders })
    };
    assert_eq!(contract.get_nonce(), 0);
    assert!(matches!(contract, VersionedContract::V0(_)));

    set_predecessor_and_deposit(alice(), NearToken::from_yoctonear(1000));
    contract.deposit();

    assert!(matches!(contract, VersionedContract::V1(_)));
    assert_eq!(contract.get_nonce(), 1);
    assert_eq!(contract.get_deposit(&alice()), Some(&NearToken::from_yoctonear(1000)));
    assert_eq!(contract.get_deposit(&bob()), Some(&NearToken::from_yoctonear(8)));
}
