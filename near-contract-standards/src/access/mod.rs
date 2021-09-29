use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, require, AccountId};
use std::collections::HashMap;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Ownable {
    pub owner: AccountId,
    pub null_owner: AccountId,
}

impl Ownable {
    pub fn new() -> Self {
        Self {
            owner: env::predecessor_account_id(),
            null_owner: "0000000000".parse::<AccountId>().unwrap(),
        }
    }

    pub fn owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn only_owner(&self) {
        require!(env::predecessor_account_id() == self.owner(), "Ownable: caller is not the owner");
    }

    pub fn renounce_ownership(&mut self) {
        self.only_owner();
        self.owner = self.null_owner.clone();
    }

    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.only_owner();
        require!(new_owner != self.null_owner, "Ownable: new owner is undefined");
        self.owner = new_owner;
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct RoleData {
    pub members: HashMap<AccountId, bool>,
    pub admin_role: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AccessControl {
    pub roles: HashMap<[u8; 32], RoleData>,
    pub null_role: [u8; 32],
}

impl AccessControl {
    pub fn new() -> Self {
        Self { roles: HashMap::new(), null_role: [0; 32] }
    }

    pub fn has_role(&self, role: &[u8; 32], account: &AccountId) -> bool {
        if !self.roles.contains_key(role) {
            return false;
        }
        *self.roles.get(role).unwrap().members.get(account).unwrap_or(&false)
    }

    pub fn check_role(&self, role: &[u8; 32], account: &AccountId) {
        if !self.has_role(role, account) {
            env::panic_str(
                format!("AccessControl: account {} is missing role {:?}", *account, *role).as_str(),
            )
        }
    }

    pub fn only_role(&self, role: &[u8; 32]) {
        self.check_role(role, &env::predecessor_account_id());
    }

    pub fn get_role_admin(&self, role: &[u8; 32]) -> [u8; 32] {
        if !self.roles.contains_key(role) {
            return self.null_role.clone();
        }
        self.roles.get(role).unwrap().admin_role.clone()
    }

    pub fn grant_role(&mut self, role: [u8; 32], account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        if !self.roles.contains_key(&role) {
            self.roles.insert(
                role,
                RoleData { members: HashMap::new(), admin_role: self.null_role.clone() },
            );
        }
        if !self.has_role(&role, &account) {
            self.roles.get_mut(&role).unwrap().members.insert(account, true);
        }
    }

    pub fn revoke_role(&mut self, role: [u8; 32], account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        if self.has_role(&role, &account) {
            self.roles.get_mut(&role).unwrap().members.insert(account, false);
        }
    }

    pub fn renounce_role(&mut self, role: [u8; 32], account: AccountId) {
        require!(
            account == env::predecessor_account_id(),
            "AccessControl: can only renounce roles for self"
        );
        self.revoke_role(role, account);
    }

    pub fn set_role_admin(&mut self, role: [u8; 32], admin_role: [u8; 32]) {
        if !self.roles.contains_key(&role) {
            self.roles.insert(
                role,
                RoleData { members: HashMap::new(), admin_role: self.null_role.clone() },
            );
        }
        self.roles.get_mut(&role).unwrap().admin_role = admin_role;
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
        assert_eq!(ownable.owner(), accounts(1));
    }
}
