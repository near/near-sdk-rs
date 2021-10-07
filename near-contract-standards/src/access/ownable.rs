/// Based on openzeppelin/access. See https://github.com/OpenZeppelin/openzeppelin-contracts/tree/master/contracts/access.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, require, AccountId};

/// Contract module which provides a basic access control mechanism, where
/// there is an account (owner) that can be granted exclusive access to
/// specific functions.
///
/// By default, the owner account will be the one that deploys the contract.
/// This can later be changed by using 'transfer_ownership'.
///
/// This module makes available the function 'only_owner', which can be applied
/// to your functions to restrict their use to the owner.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Ownable {
    pub owner: Option<AccountId>,
}

impl Ownable {
    /// Initializes the current contract by setting the caller as the initial owner.
    pub fn new() -> Self {
        Self { owner: Some(env::predecessor_account_id()) }
    }

    /// Returns the account of the current owner.  
    pub fn owner(&self) -> Option<AccountId> {
        self.owner.clone()
    }

    /// Has no effect if called by the owner.
    /// Panics otherwise.
    pub fn only_owner(&self) {
        require!(
            Some(env::predecessor_account_id()) == self.owner(),
            "Ownable: caller is not the owner"
        );
    }

    /// Permanently leaves the contract without an owner.  
    /// Can only be called by the current owner.
    ///
    /// Renouncing ownership will leave the contract without an owner,
    /// thereby removing any functionality that is only available to the owner.  
    /// ie. future calls into 'only_owner' will always panic.    
    pub fn permanently_renounce_ownership(&mut self) {
        self.only_owner();
        self.owner = None;
    }

    /// Tranfers ownership of the contract to a new account 'new_owner'.  
    /// Can only be called by the current owner.
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.only_owner();
        self.owner = Some(new_owner);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ownable = Ownable::new();
        assert_eq!(ownable.owner(), Some(accounts(1)));
    }

    #[test]
    fn test_only_owner_success() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ownable = Ownable::new();
        ownable.only_owner();
    }

    #[test]
    #[should_panic(expected = "Ownable: caller is not the owner")]
    fn test_only_owner_fail() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ownable = Ownable::new();
        let context = get_context(accounts(2));
        testing_env!(context.build());
        ownable.only_owner();
    }

    #[test]
    fn test_renounce_ownership() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ownable = Ownable::new();
        assert_eq!(ownable.owner(), Some(accounts(1)));
        ownable.permanently_renounce_ownership();
        assert_eq!(ownable.owner(), None);
    }

    #[test]
    fn test_transfer_ownership() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ownable = Ownable::new();
        assert_eq!(ownable.owner(), Some(accounts(1)));
        ownable.transfer_ownership(accounts(2));
        assert_eq!(ownable.owner(), Some(accounts(2)));
    }
}
