use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, require, AccountId};
use std::collections::HashMap;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Ownable {
    pub owner: AccountId,
    pub default_owner: AccountId,
}

impl Ownable {
    pub fn new() -> Self {
        Self {
            owner: env::predecessor_account_id(),
            default_owner: "0000000000".parse::<AccountId>().unwrap(),
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
        self.owner = self.default_owner.clone();
    }

    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.only_owner();
        require!(new_owner != self.default_owner, "Ownable: new owner is undefined");
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
    pub default_admin_role: [u8; 32],
}

impl AccessControl {
    pub fn new() -> Self {
        Self { roles: HashMap::new(), default_admin_role: [0; 32] }
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
            return self.default_admin_role.clone();
        }
        self.roles.get(role).unwrap().admin_role.clone()
    }

    fn grant_role_internal(&mut self, role: [u8; 32], account: AccountId) {
        if !self.roles.contains_key(&role) {
            self.roles.insert(
                role,
                RoleData { members: HashMap::new(), admin_role: self.default_admin_role.clone() },
            );
        }
        if !self.has_role(&role, &account) {
            self.roles.get_mut(&role).unwrap().members.insert(account, true);
        }
    }

    pub fn grant_role(&mut self, role: [u8; 32], account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        self.grant_role_internal(role, account);
    }

    pub fn setup_role(&mut self, role: [u8; 32], account: AccountId) {
        self.grant_role_internal(role, account);
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
                RoleData { members: HashMap::new(), admin_role: self.default_admin_role.clone() },
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
        assert_eq!(ownable.owner(), accounts(1));
        ownable.renounce_ownership();
        assert_eq!(ownable.owner(), "0000000000".parse::<AccountId>().unwrap());
    }

    #[test]
    fn test_transfer_ownership() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut ownable = Ownable::new();
        assert_eq!(ownable.owner(), accounts(1));
        ownable.transfer_ownership(accounts(2));
        assert_eq!(ownable.owner(), accounts(2));
    }

    #[test]
    fn test_ac_new() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        AccessControl::new();
    }

    #[test]
    fn test_ac_has_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        assert_eq!(false, ac.has_role(&[1; 32], &accounts(2)));
    }

    #[test]
    #[should_panic(
        expected = "AccessControl: account charlie is missing role [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]"
    )]
    fn test_ac_check_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        ac.check_role(&[1; 32], &accounts(2));
    }

    #[test]
    #[should_panic(
        expected = "AccessControl: account bob is missing role [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]"
    )]
    fn test_ac_only_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        ac.only_role(&[1; 32]);
    }

    #[test]
    fn test_ac_set_and_get_role_admin() {
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
