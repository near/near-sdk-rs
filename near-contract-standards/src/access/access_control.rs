use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, require, AccountId};
use std::collections::{HashMap, HashSet};

pub type RoleId = [u8; 32];

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RoleData {
    pub members: HashSet<AccountId>,
    pub admin_role: RoleId,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccessControl {
    pub roles: HashMap<RoleId, RoleData>,
    pub default_admin_role: RoleId,
}

impl AccessControl {
    pub fn new() -> Self {
        Self { roles: HashMap::new(), default_admin_role: [0; 32] }
    }

    pub fn has_role(&self, role: &RoleId, account: &AccountId) -> bool {
        if !self.roles.contains_key(role) {
            return false;
        }
        self.roles.get(role).unwrap().members.contains(account)
    }

    pub fn check_role(&self, role: &RoleId, account: &AccountId) {
        if !self.has_role(role, account) {
            env::panic_str(
                format!("AccessControl: account {} is missing role {:?}", *account, *role).as_str(),
            )
        }
    }

    pub fn only_role(&self, role: &RoleId) {
        self.check_role(role, &env::predecessor_account_id());
    }

    pub fn get_role_admin(&self, role: &RoleId) -> RoleId {
        if !self.roles.contains_key(role) {
            return self.default_admin_role.clone();
        }
        self.roles.get(role).unwrap().admin_role.clone()
    }

    fn grant_role_internal(&mut self, role: RoleId, account: AccountId) {
        if !self.roles.contains_key(&role) {
            self.roles.insert(
                role,
                RoleData { members: HashSet::new(), admin_role: self.default_admin_role.clone() },
            );
        }
        if !self.has_role(&role, &account) {
            self.roles.get_mut(&role).unwrap().members.insert(account);
        }
    }

    pub fn grant_role(&mut self, role: RoleId, account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        self.grant_role_internal(role, account);
    }

    pub fn setup_role(&mut self, role: RoleId, account: AccountId) {
        self.grant_role_internal(role, account);
    }

    pub fn revoke_role(&mut self, role: RoleId, account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        if self.has_role(&role, &account) {
            self.roles.get_mut(&role).unwrap().members.remove(&account);
        }
    }

    pub fn renounce_role(&mut self, role: RoleId, account: AccountId) {
        require!(
            account == env::predecessor_account_id(),
            "AccessControl: can only renounce roles for self"
        );
        self.revoke_role(role, account);
    }

    pub fn set_role_admin(&mut self, role: RoleId, admin_role: RoleId) {
        if !self.roles.contains_key(&role) {
            self.roles.insert(
                role,
                RoleData { members: HashSet::new(), admin_role: self.default_admin_role.clone() },
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
        AccessControl::new();
    }

    #[test]
    fn test_has_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        assert_eq!(false, ac.has_role(&[1; 32], &accounts(2)));
    }

    #[test]
    #[should_panic(
        expected = "AccessControl: account charlie is missing role [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]"
    )]
    fn test_check_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        ac.check_role(&[1; 32], &accounts(2));
    }

    #[test]
    #[should_panic(
        expected = "AccessControl: account bob is missing role [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]"
    )]
    fn test_only_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        ac.only_role(&[1; 32]);
    }

    #[test]
    fn test_set_and_get_role_admin() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ac = AccessControl::new();
        let default_admin_role = [0; 32];
        let role = [1; 32];
        let admin_role = [2; 32];
        ac.set_role_admin(role, admin_role);
        assert_eq!(admin_role, ac.get_role_admin(&role));
        assert_eq!(default_admin_role, ac.get_role_admin(&admin_role));
    }

    #[test]
    fn test_grant_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ac = AccessControl::new();
        let role = [1; 32];
        let role_admin = [2; 32];
        ac.set_role_admin(role, role_admin);
        ac.setup_role(role_admin, accounts(1));
        ac.grant_role(role, accounts(1));
        assert_eq!(true, ac.has_role(&role, &accounts(1)));
        assert_eq!(true, ac.has_role(&role, &accounts(1)));
    }

    #[test]
    #[should_panic(expected = "AccessControl: can only renounce roles for self")]
    fn test_renounce_role_fail() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ac = AccessControl::new();
        ac.renounce_role([1; 32], accounts(2));
    }

    #[test]
    fn test_renounce_role_success() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ac = AccessControl::new();
        let role = [1; 32];
        let role_admin = [2; 32];
        ac.set_role_admin(role, role_admin);
        ac.setup_role(role_admin, accounts(1));
        ac.grant_role(role, accounts(1));
        assert_eq!(true, ac.has_role(&role, &accounts(1)));
        ac.renounce_role(role, accounts(1));
        assert_eq!(false, ac.has_role(&role, &accounts(1)));
    }
}
