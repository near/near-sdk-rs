use near_sdk::json_types::{ValidAccountId, U128};

pub trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: ValidAccountId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) -> U128;
}
