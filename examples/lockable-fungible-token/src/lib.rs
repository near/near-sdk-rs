#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests;

use near_sdk::collections::UnorderedMap;
use near_sdk::{AccountId, PanicOnDefault, env, json_types::U128, near};
use std::collections::HashMap;

type Balance = u128;

#[derive(Default)]
#[near]
pub struct Account {
    /// Current unlocked balance
    pub balance: Balance,
    /// Allowed account to the allowance amount.
    pub allowances: HashMap<AccountId, Balance>,
    /// Allowed account to locked balance.
    pub locked_balances: HashMap<AccountId, Balance>,
}

impl Account {
    pub fn set_allowance(&mut self, escrow_account_id: &AccountId, allowance: Balance) {
        if allowance > 0 {
            self.allowances.insert(escrow_account_id.clone(), allowance);
        } else {
            self.allowances.remove(escrow_account_id);
        }
    }

    pub fn get_allowance(&self, escrow_account_id: &AccountId) -> Balance {
        *self.allowances.get(escrow_account_id).unwrap_or(&0)
    }

    pub fn set_locked_balance(&mut self, escrow_account_id: &AccountId, locked_balance: Balance) {
        if locked_balance > 0 {
            self.locked_balances.insert(escrow_account_id.clone(), locked_balance);
        } else {
            self.locked_balances.remove(escrow_account_id);
        }
    }

    pub fn get_locked_balance(&self, escrow_account_id: &AccountId) -> Balance {
        *self.locked_balances.get(escrow_account_id).unwrap_or(&0)
    }

    pub fn total_balance(&self) -> Balance {
        self.balance + self.locked_balances.values().sum::<Balance>()
    }
}

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct FunToken {
    /// AccountID -> Account details.
    pub accounts: UnorderedMap<AccountId, Account>,

    /// Total supply of the all token.
    pub total_supply: Balance,
}

impl FunToken {
    /// Helper method to get the account details for `owner_id`.
    fn get_account(&self, owner_id: &AccountId) -> Account {
        self.accounts.get(owner_id).unwrap_or_default()
    }
}

#[near]
impl FunToken {
    #[init]
    #[handle_result]
    pub fn new(owner_id: AccountId, total_supply: U128) -> Result<Self, &'static str> {
        let mut ft = Self { accounts: UnorderedMap::new(b"a"), total_supply: total_supply.into() };
        let mut account = ft.get_account(&owner_id);
        account.balance = total_supply.into();
        ft.accounts.insert(&owner_id, &account);
        Ok(ft)
    }

    /// Sets amount allowed to spent by `escrow_account_id` on behalf of the caller of the function
    /// (`predecessor_id`) who is considered the balance owner to the new `allowance`.
    /// If some amount of tokens is currently locked by the `escrow_account_id` the new allowance is
    /// decreased by the amount of locked tokens.
    #[handle_result]
    pub fn set_allowance(
        &mut self,
        escrow_account_id: AccountId,
        allowance: U128,
    ) -> Result<(), &'static str> {
        let allowance: u128 = allowance.into();
        let owner_id = env::predecessor_account_id();
        if escrow_account_id == owner_id {
            return Err("Can't set allowance for yourself");
        }
        let mut account = self.get_account(&owner_id);
        let locked_balance = account.get_locked_balance(&escrow_account_id);
        if locked_balance > allowance {
            return Err("The new allowance can't be less than the amount of locked tokens");
        }

        account.set_allowance(&escrow_account_id, allowance - locked_balance);
        self.accounts.insert(&owner_id, &account);

        Ok(())
    }

    /// Locks an additional `lock_amount` to the caller of the function (`predecessor_id`) from
    /// the `owner_id`.
    /// Requirements:
    /// * The (`predecessor_id`) should have enough allowance or be the owner.
    /// * The owner should have enough unlocked balance.
    #[handle_result]
    pub fn lock(&mut self, owner_id: AccountId, lock_amount: U128) -> Result<(), &'static str> {
        let lock_amount: u128 = lock_amount.into();
        if lock_amount == 0 {
            return Err("Can't lock 0 tokens");
        }
        let escrow_account_id = env::predecessor_account_id();
        let mut account = self.get_account(&owner_id);

        // Checking and updating unlocked balance
        if account.balance < lock_amount {
            return Err("Not enough unlocked balance");
        }
        account.balance -= lock_amount;

        // If locking by escrow, need to check and update the allowance.
        if escrow_account_id != owner_id {
            let allowance = account.get_allowance(&escrow_account_id);
            if allowance < lock_amount {
                return Err("Not enough allowance");
            }
            account.set_allowance(&escrow_account_id, allowance - lock_amount);
        }

        // Updating total lock balance
        let locked_balance = account.get_locked_balance(&escrow_account_id);
        account.set_locked_balance(&escrow_account_id, locked_balance + lock_amount);

        self.accounts.insert(&owner_id, &account);

        Ok(())
    }

    /// Unlocks the `unlock_amount` from the caller of the function (`predecessor_id`) back to
    /// the `owner_id`.
    /// If called not by the `owner_id` then the `unlock_amount` will be converted to the allowance.
    /// Requirements:
    /// * The (`predecessor_id`) should have at least `unlock_amount` locked tokens from `owner_id`.
    #[handle_result]
    pub fn unlock(&mut self, owner_id: AccountId, unlock_amount: U128) -> Result<(), &'static str> {
        let unlock_amount: u128 = unlock_amount.into();
        if unlock_amount == 0 {
            return Err("Can't unlock 0 tokens");
        }
        let escrow_account_id = env::predecessor_account_id();
        let mut account = self.get_account(&owner_id);

        // Checking and updating locked balance
        let locked_balance = account.get_locked_balance(&escrow_account_id);
        if locked_balance < unlock_amount {
            return Err("Not enough locked tokens");
        }
        account.set_locked_balance(&escrow_account_id, locked_balance - unlock_amount);

        // If unlocking by escrow, need to update allowance.
        if escrow_account_id != owner_id {
            let allowance = account.get_allowance(&escrow_account_id);
            account.set_allowance(&escrow_account_id, allowance + unlock_amount);
        }

        // Updating unlocked balance
        account.balance += unlock_amount;

        self.accounts.insert(&owner_id, &account);

        Ok(())
    }

    /// Transfers the `amount` of tokens from `owner_id` to the `new_owner_id`.
    /// First uses locked tokens by the caller of the function (`predecessor_id`). If the amount
    /// of locked tokens is not enough to cover the full amount, then uses unlocked tokens
    /// for the remaining balance.
    /// Requirements:
    /// * The caller of the function (`predecessor_id`) should have at least `amount` of locked plus
    /// allowance tokens.
    /// * The balance owner should have at least `amount` of locked (by `predecessor_id`) plus
    /// unlocked tokens.
    #[handle_result]
    pub fn transfer_from(
        &mut self,
        owner_id: AccountId,
        new_owner_id: AccountId,
        amount: U128,
    ) -> Result<(), &'static str> {
        let amount: u128 = amount.into();
        if amount == 0 {
            return Err("Can't transfer 0 tokens");
        }
        let escrow_account_id = env::predecessor_account_id();
        let mut account = self.get_account(&owner_id);

        // Checking and updating locked balance
        let locked_balance = account.get_locked_balance(&escrow_account_id);
        let remaining_amount = if locked_balance >= amount {
            account.set_locked_balance(&escrow_account_id, locked_balance - amount);
            0
        } else {
            account.set_locked_balance(&escrow_account_id, 0);
            amount - locked_balance
        };

        // If there is remaining balance after the locked balance, we try to use unlocked tokens.
        if remaining_amount > 0 {
            // Checking and updating unlocked balance
            if account.balance < remaining_amount {
                return Err("Not enough unlocked balance");
            }
            account.balance -= remaining_amount;

            // If transferring by escrow, need to check and update allowance.
            if escrow_account_id != owner_id {
                let allowance = account.get_allowance(&escrow_account_id);
                // Checking and updating unlocked balance
                if allowance < remaining_amount {
                    return Err("Not enough allowance");
                }
                account.set_allowance(&escrow_account_id, allowance - remaining_amount);
            }
        }

        self.accounts.insert(&owner_id, &account);

        // Deposit amount to the new owner
        let mut new_account = self.get_account(&new_owner_id);
        new_account.balance += amount;
        self.accounts.insert(&new_owner_id, &new_account);

        Ok(())
    }

    /// Same as `transfer_from` with `owner_id` `predecessor_id`.
    #[handle_result]
    pub fn transfer(&mut self, new_owner_id: AccountId, amount: U128) -> Result<(), &'static str> {
        self.transfer_from(env::predecessor_account_id(), new_owner_id, amount)
    }

    /// Returns total supply of tokens.
    pub fn get_total_supply(&self) -> U128 {
        self.total_supply.into()
    }

    /// Returns total balance for the `owner_id` account. Including all locked and unlocked tokens.
    pub fn get_total_balance(&self, owner_id: AccountId) -> U128 {
        self.get_account(&owner_id).total_balance().into()
    }

    /// Returns unlocked token balance for the `owner_id`.
    pub fn get_unlocked_balance(&self, owner_id: AccountId) -> U128 {
        self.get_account(&owner_id).balance.into()
    }

    /// Returns current allowance for the `owner_id` to be able to use by `escrow_account_id`.
    pub fn get_allowance(&self, owner_id: AccountId, escrow_account_id: AccountId) -> U128 {
        self.get_account(&owner_id).get_allowance(&escrow_account_id).into()
    }

    /// Returns current locked balance for the `owner_id` locked by `escrow_account_id`.
    pub fn get_locked_balance(&self, owner_id: AccountId, escrow_account_id: AccountId) -> U128 {
        self.get_account(&owner_id).get_locked_balance(&escrow_account_id).into()
    }
}
