use near_sdk::{ext_contract, json_types::U128, AccountId};

#[ext_contract(fungible_token_resolver_ext)]
pub trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}
