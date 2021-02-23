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
use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenMetadata, FungibleTokenMetadataProvider,
    FungibleTokenResolver,
};
use near_contract_standards::storage_manager::{AccountStorageBalance, StorageManager};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id`.
    #[init]
    pub fn new(owner_id: ValidAccountId, total_supply: U128) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let mut this = Self { token: FungibleToken::new() };
        this.token.internal_register_account(owner_id.as_ref());
        this.token.internal_deposit(owner_id.as_ref(), total_supply.into());
        this
    }
}

#[near_bindgen]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> Promise {
        self.token.ft_transfer_call(receiver_id, amount, msg, memo)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata() -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            version: String::from("1.0.0"),
            name: String::from("Example NEAR fungible token"),
            symbol: String::from("EXAMPLE"),
            reference: String::from(
                "https://github.com/near/near-sdk-rs/tree/master/examples/fungible-token",
            ),
            decimals: 24,
        }
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Contract {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        self.token.ft_resolve_transfer(sender_id, receiver_id, amount)
    }
}

#[near_bindgen]
impl StorageManager for Contract {
    #[payable]
    fn storage_deposit(&mut self, account_id: Option<ValidAccountId>) -> AccountStorageBalance {
        self.token.storage_deposit(account_id)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: U128) -> AccountStorageBalance {
        self.token.storage_withdraw(amount)
    }

    fn storage_minimum_balance(&self) -> U128 {
        self.token.storage_minimum_balance()
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> AccountStorageBalance {
        self.token.storage_balance_of(account_id)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    use super::*;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(accounts(1))
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let total_supply = 1_000_000_000_000_000u128;
        let contract = Contract::new(accounts(1).into(), total_supply.into());
        assert_eq!(contract.ft_total_supply().0, total_supply);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, total_supply);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(2));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let total_supply = 1_000_000_000_000_000u128;
        let mut contract = Contract::new(accounts(2), total_supply.into());

        println!("{:?}", contract.storage_minimum_balance());
        context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_minimum_balance().into());
        testing_env!(context.clone().predecessor_account_id(accounts(1)).build());
        contract.storage_deposit(None);

        context.storage_usage(env::storage_usage()).attached_deposit(1);
        testing_env!(context.build());
        let transfer_amount = total_supply / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0);
        testing_env!(context.build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (total_supply - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
