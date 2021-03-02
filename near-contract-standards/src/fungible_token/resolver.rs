use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::PromiseOrValue;

pub trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: ValidAccountId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) -> U128;
}
