//! Account state management

use near_account_id::AccountId;
use near_token::NearToken;
use near_vm_runner::ContractCode;
use std::collections::HashMap;
use std::sync::Arc;

/// State for a single account
#[derive(Debug, Clone)]
pub struct AccountState {
    /// Account balance
    pub balance: NearToken,
    /// Storage (key-value pairs)
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    /// Deployed contract code (if any)
    pub code: Option<Arc<ContractCode>>,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            balance: NearToken::from_near(100), // Default 100 NEAR
            storage: HashMap::new(),
            code: None,
        }
    }
}

impl AccountState {
    /// Check if this account has a contract deployed
    pub fn has_contract(&self) -> bool {
        self.code.is_some()
    }

    /// Calculate storage usage based on stored data
    pub fn calculate_storage_usage(&self) -> u64 {
        self.storage.iter().map(|(k, v)| (k.len() + v.len()) as u64).sum()
    }
}

/// Global state containing all accounts
#[derive(Debug, Default)]
pub struct GlobalState {
    /// All account states indexed by account ID
    accounts: HashMap<AccountId, AccountState>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an account
    pub fn get_or_create(&mut self, account_id: &AccountId) -> &mut AccountState {
        self.accounts.entry(account_id.clone()).or_default()
    }

    /// Get an account (returns None if not exists)
    pub fn get(&self, account_id: &AccountId) -> Option<&AccountState> {
        self.accounts.get(account_id)
    }

    /// Check if account exists
    #[allow(dead_code)]
    pub fn account_exists(&self, account_id: &AccountId) -> bool {
        self.accounts.contains_key(account_id)
    }

    /// Check if account has a contract deployed
    pub fn has_contract(&self, account_id: &AccountId) -> bool {
        self.accounts.get(account_id).is_some_and(|a| a.has_contract())
    }

    /// Deploy code to an account
    pub fn deploy(&mut self, account_id: &AccountId, code: Vec<u8>) {
        let account = self.get_or_create(account_id);
        account.code = Some(Arc::new(ContractCode::new(code, None)));
    }

    /// Get contract code for an account
    pub fn get_code(&self, account_id: &AccountId) -> Option<Arc<ContractCode>> {
        self.accounts.get(account_id)?.code.clone()
    }

    /// Get storage for an account
    pub fn get_storage(&self, account_id: &AccountId) -> Option<&HashMap<Vec<u8>, Vec<u8>>> {
        self.accounts.get(account_id).map(|a| &a.storage)
    }

    /// Get mutable storage for an account
    pub fn get_storage_mut(&mut self, account_id: &AccountId) -> &mut HashMap<Vec<u8>, Vec<u8>> {
        &mut self.get_or_create(account_id).storage
    }

    /// Set account balance
    pub fn set_balance(&mut self, account_id: &AccountId, balance: NearToken) {
        self.get_or_create(account_id).balance = balance;
    }

    /// Get account balance
    pub fn get_balance(&self, account_id: &AccountId) -> NearToken {
        self.accounts.get(account_id).map_or(NearToken::from_near(0), |a| a.balance)
    }

    /// List all account IDs
    pub fn account_ids(&self) -> impl Iterator<Item = &AccountId> {
        self.accounts.keys()
    }
}
