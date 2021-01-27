/**
* Fungible Token implementation with JSON serialization.
* NOTES:
*  - The maximum balance value is limited by U128 (2**128 - 1).
*  - JSON calls should pass U128 as a base-10 string. E.g. "100".
*  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
*    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
*  - The contract tracks the change in storage before and after the call. If the storage increases,
*    the contract requires the caller of the contract to attach enough deposit to the function call
*    to cover the storage cost.
*    This is done to prevent a denial of service attack on the contract by taking all available storage.
*    If the storage decreases, the contract will issue a refund for the cost of the released storage.
*    The unused tokens from the attached deposit are also refunded, so it's safe to
*    attach more deposit than required.
*  - To prevent the deployed contract from being modified or deleted, it should not have any access
*    keys on its account.
*/
use near_lib::token::FungibleToken as FT;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct FungibleToken {
    token: near_lib::token::Token,
}

#[near_bindgen]
impl FungibleToken {
    /// Initializes the contract with the given total supply owned by the given `owner_id`.
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self { token: near_lib::token::Token::new(owner_id, total_supply.into()) }
    }
}

#[near_bindgen]
impl FT for FungibleToken {
    fn inc_allowance(&mut self, escrow_account_id: AccountId, amount: U128) {
        self.token.inc_allowance(escrow_account_id, amount.into())
    }

    fn dec_allowance(&mut self, escrow_account_id: AccountId, amount: U128) {
        self.token.dec_allowance(escrow_account_id, amount.into())
    }

    fn transfer_from(&mut self, owner_id: AccountId, new_owner_id: AccountId, amount: U128) {
        self.token.transfer_from(owner_id, new_owner_id, amount.into())
    }

    fn transfer(&mut self, new_owner_id: AccountId, amount: U128) {
        self.token.transfer(new_owner_id, amount.into())
    }

    fn get_total_supply(&self) -> U128 {
        self.token.get_total_supply().into()
    }

    fn get_balance(&self, owner_id: AccountId) -> U128 {
        self.token.get_balance(owner_id).into()
    }

    fn get_allowance(&self, owner_id: AccountId, escrow_account_id: AccountId) -> U128 {
        self.token.get_allowance(owner_id, escrow_account_id).into()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_lib::constants::STORAGE_PRICE_PER_BYTE;
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance, VMContext};

    use super::*;

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContextBuilder::new()
            .current_account_id(accounts(0))
            .signer_account_id(accounts(1))
            .predecessor_account_id(predecessor_account_id)
            .finish()
    }

    #[test]
    fn test_new() {
        let context = get_context(accounts(2));
        testing_env!(context);
        let total_supply = 1_000_000_000_000_000u128;
        let contract = FungibleToken::new(accounts(1), total_supply.into());
        assert_eq!(contract.get_total_supply().0, total_supply);
        assert_eq!(contract.get_balance(accounts(1)).0, total_supply);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(2));
        testing_env!(context);
        let _contract = FungibleToken::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.attached_deposit = 1000 * STORAGE_PRICE_PER_BYTE;
        testing_env!(context.clone());
        let transfer_amount = total_supply / 3;
        contract.transfer(accounts(1), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_balance(accounts(2)).0, (total_supply - transfer_amount));
        assert_eq!(contract.get_balance(accounts(1)).0, transfer_amount);
    }

    #[test]
    #[should_panic(expected = "The new owner should be different from the current owner")]
    fn test_transfer_fail_self() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.attached_deposit = 1000 * STORAGE_PRICE_PER_BYTE;
        testing_env!(context.clone());
        let transfer_amount = total_supply / 3;
        contract.transfer(accounts(2), transfer_amount.into());
    }

    #[test]
    #[should_panic(expected = "Can not increment allowance for yourself")]
    fn test_self_inc_allowance_fail() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(accounts(2), (total_supply / 2).into());
    }

    #[test]
    #[should_panic(expected = "Can not decrement allowance for yourself")]
    fn test_self_dec_allowance_fail() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.dec_allowance(accounts(2), (total_supply / 2).into());
    }

    #[test]
    fn test_saturating_dec_allowance() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.dec_allowance(accounts(1), (total_supply / 2).into());
        assert_eq!(contract.get_allowance(accounts(2), accounts(1)), 0.into())
    }

    #[test]
    fn test_saturating_inc_allowance() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = std::u128::MAX;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(accounts(1), total_supply.into());
        contract.inc_allowance(accounts(1), total_supply.into());
        assert_eq!(contract.get_allowance(accounts(2), accounts(1)), std::u128::MAX.into())
    }

    #[test]
    #[should_panic(
        expected = "The required attached deposit is 12400000000000000000000, but the given attached deposit is is 0"
    )]
    fn test_self_allowance_fail_no_deposit() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.attached_deposit = 0;
        testing_env!(context.clone());
        contract.inc_allowance(accounts(1), (total_supply / 2).into());
    }

    #[test]
    fn test_carol_escrows_to_bob_transfers_to_alice() {
        // Acting as carol
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        testing_env!(context.clone());
        assert_eq!(contract.get_total_supply().0, total_supply);

        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(accounts(1), allowance.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_allowance(accounts(2), accounts(1)).0, allowance);

        // Acting as bob now
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        context.predecessor_account_id = accounts(1);
        testing_env!(context.clone());
        contract.transfer_from(accounts(2), accounts(0), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_balance(accounts(2)).0, total_supply - transfer_amount);
        assert_eq!(contract.get_balance(accounts(0)).0, transfer_amount);
        assert_eq!(contract.get_allowance(accounts(2), accounts(1)).0, allowance - transfer_amount);
    }

    #[test]
    fn test_carol_escrows_to_bob_locks_and_transfers_to_alice() {
        // Acting as carol
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.storage_usage = env::storage_usage();

        context.is_view = true;
        testing_env!(context.clone());
        assert_eq!(contract.get_total_supply().0, total_supply);

        let allowance = total_supply / 3;
        let transfer_amount = allowance / 3;
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(accounts(1), allowance.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_allowance(accounts(2), accounts(1)).0, allowance);
        assert_eq!(contract.get_balance(accounts(2)).0, total_supply);

        // Acting as bob now
        context.is_view = false;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        context.predecessor_account_id = accounts(1);
        testing_env!(context.clone());
        contract.transfer_from(accounts(2), accounts(0), transfer_amount.into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();

        context.is_view = true;
        context.attached_deposit = 0;
        testing_env!(context.clone());
        assert_eq!(contract.get_balance(accounts(2)).0, (total_supply - transfer_amount));
        assert_eq!(contract.get_balance(accounts(0)).0, transfer_amount);
        assert_eq!(contract.get_allowance(accounts(2), accounts(1)).0, allowance - transfer_amount);
    }

    #[test]
    fn test_self_allowance_set_for_refund() {
        let mut context = get_context(accounts(2));
        testing_env!(context.clone());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = FungibleToken::new(accounts(2), total_supply.into());
        context.storage_usage = env::storage_usage();

        let initial_balance = context.account_balance;
        let initial_storage = context.storage_usage;
        context.attached_deposit = STORAGE_PRICE_PER_BYTE * 1000;
        testing_env!(context.clone());
        contract.inc_allowance(accounts(1), (total_supply / 2).into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();
        assert_eq!(
            context.account_balance,
            initial_balance
                + Balance::from(context.storage_usage - initial_storage) * STORAGE_PRICE_PER_BYTE
        );

        let initial_balance = context.account_balance;
        let initial_storage = context.storage_usage;
        testing_env!(context.clone());
        context.attached_deposit = 0;
        testing_env!(context.clone());
        contract.dec_allowance(accounts(1), (total_supply / 2).into());
        context.storage_usage = env::storage_usage();
        context.account_balance = env::account_balance();
        assert!(context.storage_usage < initial_storage);
        assert!(context.account_balance < initial_balance);
        assert_eq!(
            context.account_balance,
            initial_balance
                - Balance::from(initial_storage - context.storage_usage) * STORAGE_PRICE_PER_BYTE
        );
    }
}
