/// Based on openzeppelin/access. See https://github.com/OpenZeppelin/openzeppelin-contracts/tree/master/contracts/access.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, require, AccountId};

use sha3::{Digest, Keccak256};
use std::collections::{HashMap, HashSet};

pub type RoleId = [u8; 32];

pub fn keccak256(text: String) -> RoleId {
    let mut hasher = Keccak256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    result.into()
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RoleData {
    pub members: HashSet<AccountId>,
    pub admin_role: RoleId,
}

/// Contract module that allows accounts to implement role-based access
/// control mechanisms.
///
/// Roles are referred to by their 'RoleId' identifier ([u8; 32]). They
/// should be exposed in the contract and be unique. The best way to
/// achieve this is by using 'keccak256' function:
/*
    let MY_ROLE: [u8; 32] = keccak256(String::from("MY_ROLE"));
*/
///
///
/// Roles can be used to represent a set of permissions. To restrict access to a
/// function call, use 'has_role':
/*
    pub fn foo() {
      require!(has_role(&MY_ROLE, &env::predecessor_account_id()));
      ...
    }
*/
///
/// Roles can be granted and revoked dynamically via the 'grant_role' and
/// 'revoke_role' functions. Each role has an associated admin role, and only
/// accounts that have a role's admin role can call 'grant_role' and 'revoke_role'.
///
/// By default, the admin role for all roles is 'default_admin_role', which means
/// that only accounts with this role will be able to grant or revoke other
/// roles. More complex role relationships can be created by using
/// 'internal_set_role_admin'.
///
/// WARNING: The 'default_admin_role' is also its own admin: it has permission to
/// grant and revoke this role. Extra precautions should be taken to secure
/// accounts that have been granted it.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AccessControl {
    pub roles: HashMap<RoleId, RoleData>,
    pub default_admin_role: RoleId,
}

impl AccessControl {
    pub fn new() -> Self {
        Self { roles: HashMap::new(), default_admin_role: RoleId::default() }
    }

    /// Returns true if 'account' has been granted the specific 'role'.  
    /// Returns false otherwise.
    pub fn has_role(&self, role: &RoleId, account: &AccountId) -> bool {
        if !self.roles.contains_key(role) {
            return false;
        }
        self.roles.get(role).unwrap().members.contains(account)
    }

    /// Has no effect if 'account' has been granted the specific 'role'.  
    /// Otherwise, panics with a standard message.
    ///
    /// Uses 'has_role' internally.
    pub fn internal_check_role(&self, role: &RoleId, account: &AccountId) {
        if !self.has_role(role, account) {
            env::panic_str(
                format!("AccessControl: account {} is missing role {:?}", *account, *role).as_str(),
            )
        }
    }

    /// Has no effect if the caller has been granted the specific 'role'.  
    /// Otherwise, panics with a standard message.
    ///
    /// Uses 'internal_check_role' internally.
    pub fn only_role(&self, role: &RoleId) {
        self.internal_check_role(role, &env::predecessor_account_id());
    }

    /// Returns the admin for a specific 'role'.  
    /// If 'role' does not exist, returns the default admin.
    ///
    /// See 'internal_set_role_admin' to change a role's admin.
    pub fn get_role_admin(&self, role: &RoleId) -> RoleId {
        if !self.roles.contains_key(role) {
            return self.default_admin_role.clone();
        }
        self.roles.get(role).unwrap().admin_role.clone()
    }

    /// Grants a specific 'role' to a specific 'account'.
    /// If the 'role' does not exist in the current system,
    /// it also creates the 'role'.
    ///
    /// This method should only be called when setting
    /// up the initial roles for the system.
    ///
    /// Using this function in any other way is effectively circumventing the admin
    /// system imposed by this module.
    ///
    /// Uses 'has_role' internally.
    pub fn internal_setup_role(&mut self, role: RoleId, account: AccountId) {
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

    /// Grants a specific 'role' to a specific 'account'.
    ///
    /// Requirements:
    ///
    /// - The role must exist.
    /// - The caller must have the role's admin.
    ///
    /// Uses 'only_role' and then 'internal_setup_role' internally.  
    ///
    /// See also 'revoke_role' for the opposite effect.
    pub fn grant_role(&mut self, role: RoleId, account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        self.internal_setup_role(role, account);
    }

    /// Revokes a specific 'role' from a specific 'account'.
    ///
    /// Requirements:
    /// - The role must exist.
    /// - The caller must have the role's admin.
    ///
    /// Uses 'only_role' and 'has_role' internally.  
    ///
    /// See also 'grant_role' for the opposite effect.
    pub fn revoke_role(&mut self, role: RoleId, account: AccountId) {
        self.only_role(&self.get_role_admin(&role));
        if self.has_role(&role, &account) {
            self.roles.get_mut(&role).unwrap().members.remove(&account);
        }
    }

    /// Takes away a specific 'role' from a specific 'account'.  
    /// It has no effect if the 'role' does not exist, or if the specified 'account'
    /// is not enabled for that specific 'role'.
    ///
    /// Panics if the specified 'account' is not the account of the caller.
    ///
    /// This method's purpose is to provide a mechanism for accounts to purposefuly
    /// lose their own privileges, such as when they are compromised
    /// (eg. when a trusted device is misplaced).
    pub fn renounce_role(&mut self, role: RoleId, account: AccountId) {
        require!(
            account == env::predecessor_account_id(),
            "AccessControl: can only renounce roles for self"
        );
        self.revoke_role(role, account);
    }

    /// Sets 'admin_role' as 'role's admin, creating the 'role' if necessary.  
    ///
    /// There are no further verifications about the caller.
    pub fn internal_set_role_admin(&mut self, role: RoleId, admin_role: RoleId) {
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
    fn test_internal_check_role() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let ac = AccessControl::new();
        ac.internal_check_role(&[1; 32], &accounts(2));
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
        ac.internal_set_role_admin(role, admin_role);
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
        ac.internal_set_role_admin(role, role_admin);
        ac.internal_setup_role(role_admin, accounts(1));
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
        ac.internal_set_role_admin(role, role_admin);
        ac.internal_setup_role(role_admin, accounts(1));
        ac.grant_role(role, accounts(1));
        assert_eq!(true, ac.has_role(&role, &accounts(1)));
        ac.renounce_role(role, accounts(1));
        assert_eq!(false, ac.has_role(&role, &accounts(1)));
    }
}
