use borsh::{BorshDeserialize, BorshSerialize};
use near_bindgen::collections::Map;
use near_bindgen::{env, metadata, near_bindgen, AccountId, Balance};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    /// Current balance.
    pub balance: Balance,
    /// Allowed account to the allowance amount.
    pub allowances: Map<Vec<u8>, Balance>,
}

impl Account {
    pub fn new(account_hash: Vec<u8>) -> Self {
        Self { balance: 0, allowances: Map::new(account_hash) }
    }

    pub fn set_allowance(&mut self, escrow_account_id: &AccountId, allowance: Balance) {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        if allowance > 0 {
            self.allowances.insert(&escrow_hash, &allowance);
        } else {
            self.allowances.remove(&escrow_hash);
        }
    }

    pub fn get_allowance(&self, escrow_account_id: &AccountId) -> Balance {
        let escrow_hash = env::sha256(escrow_account_id.as_bytes());
        self.allowances.get(&escrow_hash).unwrap_or(0)
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct FunToken {
    /// AccountID -> Account details.
    pub accounts: Map<Vec<u8>, Account>,

    /// Total supply of the all token.
    pub total_supply: Balance,
}

impl Default for FunToken {
    fn default() -> Self {
        panic!("Fun token should be initialized before usage")
    }
}

#[near_bindgen]
impl FunToken {
    #[init]
    pub fn new(
        #[serializer(borsh)] owner_id: AccountId,
        #[serializer(borsh)] total_supply: Balance,
    ) -> Self {
        assert!(env::state_read::<Self>().is_none(), "Already initialized");
        let mut ft = Self { accounts: Map::new(b"a".to_vec()), total_supply };
        let mut account = ft.get_account(&owner_id);
        account.balance = total_supply;
        ft.set_account(&owner_id, &account);
        ft
    }

    /// Sets amount allowed to spent by `escrow_account_id` on behalf of the caller of the function
    /// (`predecessor_id`) who is considered the balance owner to the new `allowance`.
    pub fn set_allowance(
        &mut self,
        #[serializer(borsh)] escrow_account_id: AccountId,
        #[serializer(borsh)] allowance: Balance,
    ) {
        let owner_id = env::predecessor_account_id();
        if escrow_account_id == owner_id {
            env::panic(b"Can't set allowance for yourself");
        }
        let mut account = self.get_account(&owner_id);

        account.set_allowance(&escrow_account_id, allowance);
        self.set_account(&owner_id, &account);
    }

    /// Transfers the `amount` of tokens from `owner_id` to the `new_owner_id`.
    /// Requirements:
    /// * `amount` should be a positive integer.
    /// * `owner_id` should have balance on the account greater or equal to the transfer `amount`.
    /// * If this function is called by an escrow account (`owner_id != predecessor_account_id`),
    ///   then the allowance of the caller of the function (`predecessor_account_id`) on
    ///   the account of `owner_id` should be greater or equal to the transfer `amount`.
    pub fn transfer_from(
        &mut self,
        #[serializer(borsh)] owner_id: AccountId,
        #[serializer(borsh)] new_owner_id: AccountId,
        #[serializer(borsh)] amount: Balance,
    ) {
        if amount == 0 {
            env::panic(b"Can't transfer 0 tokens");
        }
        let mut account = self.get_account(&owner_id);

        // Checking and updating unlocked balance
        if account.balance < amount {
            env::panic(b"Not enough balance");
        }
        account.balance -= amount;

        // If transferring by escrow, need to check and update allowance.
        let escrow_account_id = env::predecessor_account_id();
        if escrow_account_id != owner_id {
            let allowance = account.get_allowance(&escrow_account_id);
            // Checking and updating unlocked balance
            if allowance < amount {
                env::panic(b"Not enough allowance");
            }
            account.set_allowance(&escrow_account_id, allowance - amount);
        }

        self.set_account(&owner_id, &account);

        // Deposit amount to the new owner
        let mut new_account = self.get_account(&new_owner_id);
        new_account.balance += amount;
        self.set_account(&new_owner_id, &new_account);
    }

    /// Same as `transfer_from` with `owner_id` `predecessor_id`.
    pub fn transfer(
        &mut self,
        #[serializer(borsh)] new_owner_id: AccountId,
        #[serializer(borsh)] amount: Balance,
    ) {
        self.transfer_from(env::predecessor_account_id(), new_owner_id, amount);
    }

    /// Returns total supply of tokens.
    #[result_serializer(borsh)]
    pub fn get_total_supply(&self) -> Balance {
        self.total_supply
    }

    /// Returns total balance for the `owner_id` account. Including all locked and unlocked tokens.
    #[result_serializer(borsh)]
    pub fn get_balance(&self, #[serializer(borsh)] owner_id: AccountId) -> Balance {
        self.get_account(&owner_id).balance
    }

    /// Returns current allowance for the `owner_id` to be able to use by `escrow_account_id`.
    #[result_serializer(borsh)]
    pub fn get_allowance(
        &self,
        #[serializer(borsh)] owner_id: AccountId,
        #[serializer(borsh)] escrow_account_id: AccountId,
    ) -> Balance {
        self.get_account(&owner_id).get_allowance(&escrow_account_id)
    }
}

impl FunToken {
    /// Helper method to get the account details for `owner_id`.
    fn get_account(&self, owner_id: &AccountId) -> Account {
        let account_hash = env::sha256(owner_id.as_bytes());
        self.accounts.get(&account_hash).unwrap_or_else(|| Account::new(account_hash))
    }

    /// Helper method to get the account details for `owner_id`.
    fn set_account(&mut self, owner_id: &AccountId, account: &Account) {
        let account_hash = env::sha256(owner_id.as_bytes());
        self.accounts.insert(&account_hash, &account);
    }
}

metadata! {}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use near_bindgen::MockedBlockchain;
    use near_bindgen::{testing_env, VMContext};

    use super::*;

    fn alice() -> AccountId {
        "alice.near".to_string()
    }
    fn bob() -> AccountId {
        "bob.near".to_string()
    }
    fn carol() -> AccountId {
        "carol.near".to_string()
    }

    fn catch_unwind_silent<F: FnOnce() -> R + std::panic::UnwindSafe, R>(
        f: F,
    ) -> std::thread::Result<R> {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(f);
        std::panic::set_hook(prev_hook);
        result
    }

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: bob(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
        }
    }

    #[test]
    fn test_new() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        let contract = FunToken::new(bob(), total_supply);
        assert_eq!(contract.get_total_supply(), total_supply);
        assert_eq!(contract.get_balance(bob()), total_supply);
    }

    #[test]
    fn test_transfer() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FunToken::new(carol(), total_supply);
        let transfer_amount = total_supply / 3;
        contract.transfer(bob(), transfer_amount);
        assert_eq!(contract.get_balance(carol()), (total_supply - transfer_amount));
        assert_eq!(contract.get_balance(bob()), transfer_amount);
    }

    #[test]
    fn test_self_allowance_fail() {
        let context = get_context(carol());
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FunToken::new(carol(), total_supply);
        catch_unwind_silent(move || {
            contract.set_allowance(carol(), total_supply / 2);
        })
        .unwrap_err();
    }

    #[test]
    fn test_carol_escrows_to_bob_transfers_to_alice() {
        // Acting as carol
        testing_env!(get_context(carol()));
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FunToken::new(carol(), total_supply);
        assert_eq!(contract.get_total_supply(), total_supply);
        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        contract.set_allowance(bob(), allowance);
        assert_eq!(contract.get_allowance(carol(), bob()), allowance);
        // Acting as bob now
        testing_env!(get_context(bob()));
        contract.transfer_from(carol(), alice(), transfer_amount);
        assert_eq!(contract.get_balance(carol()), total_supply - transfer_amount);
        assert_eq!(contract.get_balance(alice()), transfer_amount);
        assert_eq!(contract.get_allowance(carol(), bob()), allowance - transfer_amount);
    }

    #[test]
    fn test_carol_escrows_to_bob_locks_and_transfers_to_alice() {
        // Acting as carol
        testing_env!(get_context(carol()));
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FunToken::new(carol(), total_supply);
        assert_eq!(contract.get_total_supply(), total_supply);
        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        contract.set_allowance(bob(), allowance);
        assert_eq!(contract.get_allowance(carol(), bob()), allowance);
        // Acting as bob now
        testing_env!(get_context(bob()));
        assert_eq!(contract.get_balance(carol()), total_supply);
        contract.transfer_from(carol(), alice(), transfer_amount);
        assert_eq!(contract.get_balance(carol()), (total_supply - transfer_amount));
        assert_eq!(contract.get_balance(alice()), transfer_amount);
        assert_eq!(contract.get_allowance(carol(), bob()), allowance - transfer_amount);
    }
}
