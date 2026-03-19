use std::num::NonZeroU128;

use near_token::NearToken;

/// Allow an access key to spend either an unlimited or limited amount of gas
// This wrapper prevents incorrect construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Allowance {
    Unlimited,
    Limited(NonZeroU128),
}

impl Allowance {
    pub fn unlimited() -> Allowance {
        Allowance::Unlimited
    }

    /// This will return an None if you try to pass a zero value balance
    pub fn limited(balance: NearToken) -> Option<Allowance> {
        NonZeroU128::new(balance.as_yoctonear()).map(Allowance::Limited)
    }
}
