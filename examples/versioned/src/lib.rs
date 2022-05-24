use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::store::UnorderedMap;
use near_sdk::{env, log, near_bindgen, AccountId, Balance};

/// An example of a versioned contract. This is a simple contract that tracks how much
/// each account deposits into the contract. In v1, a nonce is added to state which increments
/// after each successful deposit.
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub enum VersionedContract {
    V0(ContractV0),
    V1(Contract),
}

impl VersionedContract {
    fn contract_mut(&mut self) -> &mut Contract {
        let old_contract = match self {
            Self::V1(contract) => return contract,
            Self::V0(contract) => {
                // Contract state is old version, take old state to upgrade.
                core::mem::take(contract)
            }
        };

        // Upgrade state of self and return mutable reference to it.
        *self = Self::V1(Contract { funders: old_contract.funders, nonce: 0 });
        if let Self::V1(contract) = self {
            contract
        } else {
            // Variant is constructed above, this is unreachable
            env::abort()
        }
    }

    fn funders(&self) -> &UnorderedMap<AccountId, Balance> {
        match self {
            Self::V0(contract) => &contract.funders,
            Self::V1(contract) => &contract.funders,
        }
    }
}

impl Default for VersionedContract {
    fn default() -> Self {
        VersionedContract::V1(Contract::default())
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractV0 {
    funders: UnorderedMap<AccountId, Balance>,
}

impl Default for ContractV0 {
    fn default() -> Self {
        Self { funders: UnorderedMap::new(b"f") }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    funders: UnorderedMap<AccountId, Balance>,
    nonce: u64,
}

impl Default for Contract {
    fn default() -> Self {
        Self { funders: UnorderedMap::new(b"f"), nonce: 0 }
    }
}

#[near_bindgen]
impl VersionedContract {
    #[payable]
    pub fn deposit(&mut self) {
        let account_id = env::predecessor_account_id();
        let deposit = env::attached_deposit();
        log!("{} deposited {} yNEAR", account_id, deposit);
        let contract = self.contract_mut();
        *contract.funders.entry(account_id).or_default() += deposit;
        contract.nonce += 1;
    }

    pub fn get_nonce(&self) -> u64 {
        match self {
            Self::V0(_) => 0,
            Self::V1(contract) => contract.nonce,
        }
    }

    pub fn get_deposit(&self, account_id: &AccountId) -> Option<&Balance> {
        self.funders().get(account_id)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::{alice, bob};
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn set_predecessor_and_deposit(predecessor: AccountId, deposit: Balance) {
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .attached_deposit(deposit)
            .build())
    }

    #[test]
    fn basic() {
        let mut contract = VersionedContract::default();
        set_predecessor_and_deposit(bob(), 8);
        contract.deposit();

        set_predecessor_and_deposit(alice(), 10);
        contract.deposit();

        set_predecessor_and_deposit(bob(), 20);
        contract.deposit();

        assert_eq!(contract.get_deposit(&alice()), Some(&10));
        assert_eq!(contract.get_deposit(&bob()), Some(&28));
        assert_eq!(contract.get_nonce(), 3);
    }

    #[test]
    fn contract_v0_interactions() {
        let mut contract = {
            let mut funders = UnorderedMap::new(b"f");
            funders.insert(bob(), 8);
            VersionedContract::V0(ContractV0 { funders })
        };
        assert_eq!(contract.get_nonce(), 0);
        assert!(matches!(contract, VersionedContract::V0(_)));

        set_predecessor_and_deposit(alice(), 1000);
        contract.deposit();

        assert!(matches!(contract, VersionedContract::V1(_)));
        assert_eq!(contract.get_nonce(), 1);
        assert_eq!(contract.get_deposit(&alice()), Some(&1000));
        assert_eq!(contract.get_deposit(&bob()), Some(&8));
    }
}
