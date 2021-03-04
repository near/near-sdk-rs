/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, ValidAccountId, U128};
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    name: Option<String>,
    symbol: Option<String>,
    icon: Option<String>,
    reference: Option<String>,
    reference_hash: Option<Base64VecU8>,
    decimals: Option<u8>,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id`.
    #[init]
    pub fn new(
        owner_id: ValidAccountId,
        total_supply: U128,
        name: Option<String>,
        symbol: Option<String>,
        icon: Option<String>,
        reference: Option<String>,
        reference_hash: Option<Base64VecU8>,
        decimals: Option<u8>
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        if reference_hash.is_some() {
            assert_eq!(reference_hash.clone().unwrap().0.len(), 32, "Hash has to be 32 bytes");
        }
        let valid_params = near_contract_standards::fungible_token::metadata::are_valid_metadata_params(name.clone(), symbol.clone(), icon.clone(), reference.clone(), reference_hash.clone(), decimals.clone());
        assert!(valid_params, "Ensure you have provided all required metadata parameters: name, symbol, and decimals. Note that metadata fields are required.");

        let mut this = Self {
            token: FungibleToken::new(b"a"),
            name,
            symbol,
            icon,
            reference,
            reference_hash,
            decimals
        };
        this.token.internal_register_account(owner_id.as_ref());
        this.token.internal_deposit(owner_id.as_ref(), total_supply.into());
        this
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_ar!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: self.name.clone().unwrap(),
            symbol: self.symbol.clone().unwrap(),
            icon: self.icon.clone(),
            reference: self.reference.clone(),
            reference_hash: self.reference_hash.clone(),
            decimals: self.decimals.clone().unwrap(),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const REFERENCE: &str =
        "https://github.com/near/near-sdk-rs/tree/master/examples/fungible-token";
    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new(
            accounts(1).into(),
            TOTAL_SUPPLY.into(),
            Some("Mochi Rewards".to_string()),
            Some("MOCHI".to_string()),
            Some("https://example.com/mochi.svg".to_string()),
            Some(REFERENCE.to_string()),
            Some(Base64VecU8::from(vec![1; 32])),
            Some(24u8)
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new(
            accounts(2).into(),
            TOTAL_SUPPLY.into(),
            Some("Mochi Rewards".to_string()),
            Some("MOCHI".to_string()),
            Some("https://example.com/mochi.svg".to_string()),
            Some(REFERENCE.to_string()),
            Some(Base64VecU8::from(vec![1; 32])),
            Some(24u8)
        );
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.ar_registration_fee().into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.ar_register(None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
